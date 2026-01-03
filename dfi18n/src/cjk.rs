use crate::glyph;

// Check if a character is a CJK (Chinese, Japanese, Korean) character
pub fn is_cjk(ch: char) -> bool {
  let (w, h) = glyph::get_glyph_surface(ch).get_size();
  w == h
}

// Check if a character is a common CJK punctuation mark
pub fn is_cjk_punctuation(ch: char) -> bool {
  match ch {
    '。' | '，' | '？' | '！' => true,
    _ => false,
  }
}
