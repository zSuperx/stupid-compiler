fn better(n: *u64) {
  if *n < 69 {
    *n = 67;
  } else if *n > 69 {
    *n = 420;
  }
}

fn main() -> u32 {
  let my_number: u8 = 53;

  let n = @u64 my_number;
  better(&n);

  return 0;
}
