pub fn strip_utf8_bom(s: &str) -> &str {
  if let Some(c) = s.get(0..=2) {
    if c.as_bytes() == &[0xef, 0xbb, 0xbf] {
      return &s[3..];
    }
  }
  s
}
