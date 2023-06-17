use dace::ast::{Node, Stmt, LoopBound, AryRef};
use dace::arybase::set_arybase;
use lru_stack::LRUStack;
use hist::Hist;
use std::rc::Rc;

fn access2addr(ary_ref: &AryRef, ivec: &Vec<i32>) -> usize {

    let ary_index = (ary_ref.sub)(ivec);
    if ary_index.len() != ary_ref.dim.len() { panic!("array index and dimension do not match"); }
    
    let offset = ary_index.iter().zip(ary_ref.dim.iter())
	.fold(0, |acc, (&i, &d)| acc*d + i);
    
    return ary_ref.base.unwrap() + offset;
}

fn trace_rec(code: &Rc<Node>, ivec: &Vec<i32>, sim: &mut LRUStack<usize>, hist: &mut Hist) {
    match &code.stmt {
	Stmt::Ref(ary_ref) => {let addr = access2addr(&ary_ref, &ivec);
			       let rd = sim.rec_access_impl(addr);
			       hist.add_dist(rd);},
	Stmt::Loop(aloop) => {
	    if let LoopBound::Fixed(lb) = aloop.lb {
		if let LoopBound::Fixed(ub) = aloop.ub {
		    (lb..ub).into_iter().for_each(
			|i| {
			aloop.body.iter().for_each(
			    |stmt| {
				let mut myvec = ivec.clone();
				myvec.push(i);
				trace_rec(stmt, &myvec, sim, hist) })})
		}
		else {panic!("dynamic loop upper bound is not supported")}
	    }
	    else {panic!("dynamic loop lower bound is not supported")}
	},
	Stmt::Block(blk) => blk.iter().for_each( |s| trace_rec(s, &ivec.clone(), sim, hist) )
    }
}

pub fn trace(code: &mut Rc<Node>) -> Hist {
    let mut sim = LRUStack::<usize>::new();
    let mut hist = Hist::new();
    set_arybase( code );
    println!("{:?}", code);
    trace_rec(code, &Vec::<i32>::new(), &mut sim, &mut hist);
    hist
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_access2addr() {
	let mut aij_node = Node::new_ref("x", vec![10,10],
					      |ij| vec![ij[0] as usize, ij[1] as usize]);
	let mutable = unsafe{ Rc::get_mut_unchecked( &mut aij_node ) };
	*mutable.ref_only_mut_ref( |a| &mut a.base ).unwrap() = Some(0);
	if let Stmt::Ref(aij) = &aij_node.stmt {
	    assert_eq!(access2addr(&aij, &vec![0,0]), 0);
	    assert_eq!(access2addr(&aij, &vec![9,9]), 99);
	}
    }

    #[test]
    fn loop_a_i() {

        // i = 0, n { a[i] }
        let mut aref = Node::new_ref("A", vec![10],
				      |i| vec![i[0] as usize]);
        let mut aloop = Node::new_single_loop("i", 0, 10);
        Node::extend_loop_body(&mut aloop, &mut aref);

	let hist = trace(&mut aloop);
	assert_eq!(hist.to_vec()[0], (None, 10));
	println!("{}", hist);	
    }

    #[test]
    fn loop_a_0() {

        // i = 0, n { a[0] }
        let mut aref = Node::new_ref("A", vec![1], |_| vec![0]);
        let mut aloop = Node::new_single_loop("i", 0, 10);
        Node::extend_loop_body(&mut aloop, &mut aref);

	let hist = trace(&mut aloop);
	assert_eq!(hist.to_vec()[0], (Some(1), 9));
	assert_eq!(hist.to_vec()[1], (None, 1));
	println!("{}", hist);	
    }
}
