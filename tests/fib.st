fn fib(n: u32) -> u32 {
  if n < 2 {
    return n;
  } else {
    return fib(n - 2) + fib(n - 1);
  }
}

fn main(argc: u8, argv: **u8) {
  fib(@(u32)**argv);
}
