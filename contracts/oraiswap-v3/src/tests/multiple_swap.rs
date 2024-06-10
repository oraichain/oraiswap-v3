use crate::tests::helper::{macros::*, MockApp};

#[test]
fn test_multiple_swap_x_to_y() {
    let mut app = MockApp::new(&[("alice", &[])]);
    multiple_swap!(app, true);
}

#[test]
fn test_multiple_swap_y_to_x() {
    let mut app = MockApp::new(&[("alice", &[])]);
    multiple_swap!(app, false);
}
