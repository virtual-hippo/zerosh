pub fn eprintln_and_return_false(msg: &str) -> bool {
    eprintln!("{msg}");
    false
}

pub struct CleanUp<F>
where
    F: Fn(),
{
    f: F,
}

impl<F> CleanUp<F>
where
    F: Fn(),
{
    pub fn new(f: F) -> Self {
        CleanUp {
            f
        }
    }

}

impl<F> Drop for CleanUp<F>
where
    F: Fn(),
{
    fn drop(&mut self) {
        (self.f)()
    }
}
