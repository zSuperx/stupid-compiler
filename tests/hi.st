fn bob() -> i32 {
  return 69 * 420;
}

fn bob2() {
  let x = 5;
}

fn main(y: i32) -> i32 {
  if y > 9 {
    return y;
  } else {
    y = 69;
  }
  return main(y + 1);
}
