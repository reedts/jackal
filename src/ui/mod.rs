pub mod calview;

pub trait Selection {
    fn move_left(&mut self);
    fn move_right(&mut self);
    fn move_up(&mut self);
    fn move_down(&mut self);

    fn move_n_left(&mut self, n: usize);
    fn move_n_right(&mut self, n: usize);
    fn move_n_up(&mut self, n: usize);
    fn move_n_down(&mut self, n: usize);
}
