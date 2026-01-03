use core::ffi;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{char, mem, ptr};

use crate::{cjk, df, glyph, hooks, text, types};

// Reset all managed markups text boxes
pub fn reset() {
  get_markups_mut().clear();
}

// All managed markups text boxes maps by their content string
static MARKUPS: OnceLock<RwLock<HashMap<String, ManagedMarkupTextBox>>> = OnceLock::new();

// Getting mutable access to the managed markups text boxes
pub fn get_markups_mut() -> RwLockWriteGuard<'static, HashMap<String, ManagedMarkupTextBox>> {
  MARKUPS.get_or_init(|| RwLock::new(HashMap::new())).write().unwrap()
}

// All markup text boxes maps by their address to original content string
static MTBS: OnceLock<RwLock<HashMap<usize, String>>> = OnceLock::new();

// Getting access to the markup text boxes
pub fn get_mtbs() -> RwLockReadGuard<'static, HashMap<usize, String>> {
  MTBS.get_or_init(|| RwLock::new(HashMap::new())).read().unwrap()
}

// Getting mutable access to the markup text boxes
pub fn get_mtbs_mut() -> RwLockWriteGuard<'static, HashMap<usize, String>> {
  MTBS.get_or_init(|| RwLock::new(HashMap::new())).write().unwrap()
}

#[allow(non_camel_case_types)]
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum LinkType {
  NONE = -1,
  HIST_FIG = 0,
  SITE = 1,
  ARTIFACT = 2,
  BOOK = 3,
  SUBREGION = 4,
  FEATURE_LAYER = 5,
  ENTITY = 6,
  ABSTRACT_BUILDING = 7,
  ENTITY_POPULATION = 8,
  ART_IMAGE = 9,
  ERA = 10,
  HEC = 11,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
#[allow(unused)]
pub enum CursesColor {
  Black = 0x0,
  Blue = 0x1,
  Green = 0x2,
  Aqua = 0x3,
  Red = 0x4,
  Purple = 0x5,
  Yellow = 0x6,
  White = 0x7,
  Gray = 0x8,
  LightBlue = 0x9,
  LightGreen = 0xa,
  LightAqua = 0xb,
  LightRed = 0xc,
  LightPurple = 0xd,
  LightYellow = 0xe,
  BrightWhite = 0xf,
}

impl From<i32> for CursesColor {
  fn from(value: i32) -> Self {
    unsafe { std::mem::transmute::<i32, CursesColor>(value & 0xf) }
  }
}

impl CursesColor {
  pub fn light(self) -> Self {
    ((self as i32) & 0x7 | 0x8).into()
  }
  pub fn dark(self) -> Self {
    ((self as i32) & 0x7).into()
  }
  pub fn with_bright(self, bright: bool) -> Self {
    if bright { self.light() } else { self.dark() }
  }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct ManagedMarkupLink {
  typ: LinkType,
  id: i32,
  subid: i32,
}

impl ManagedMarkupLink {
  fn new(typ: LinkType, id: i32, subid: i32) -> Self {
    Self { typ, id, subid }
  }
}

const MARKUP_WORD_FLAG_NEWLINE: u32 = 0b0001;
const MARKUP_WORD_FLAG_BLANK_LINE: u32 = 0b0010;
const MARKUP_WORD_FLAG_INDENT: u32 = 0b0100;

#[derive(Debug, Default, Clone)]
pub struct ManagedMarkupWord {
  str: String,
  color: types::Color,
  link_index: i32,
  x: i32,        // layout x in column
  y: i32,        // layout y in column
  width: i32,    // layout width in column
  width_px: i32, // cached width in pixels
  flags: u32,
}

impl Into<text::TextFragment> for ManagedMarkupWord {
  fn into(self) -> text::TextFragment {
    let content = self.str.clone();
    let color_pair = types::ColorPair::new_foreground(self.color);
    text::TextFragment::new(content, color_pair)
  }
}

#[derive(Default, Clone)]
pub struct ManagedMarkupTextBox {
  pub word: Vec<ManagedMarkupWord>,
  link: Vec<ManagedMarkupLink>,
  current_width: i32,
  max_y: i32,
  // The text block layout variables
  text_block: text::TextBlock,
  current_row: text::TextRow,
  remain_width_px: i32,
  x_px: i32,
  y_val: i32,
  occupied_x: i32,
  word_x_start: i32,
  word_x_end: i32,
}

impl ManagedMarkupTextBox {
  // Parse markup text into a MarkupTextBox
  // From dfhack/library/modules/Gui.cpp: Gui::MTB_parse()
  pub fn parse(content: &str) -> Self {
    let mut mtb: ManagedMarkupTextBox = Default::default();

    let chars = content.chars().collect::<Vec<char>>();

    // handle empty string
    if chars.is_empty() {
      let mut mw = ManagedMarkupWord::default();
      mw.flags |= MARKUP_WORD_FLAG_NEWLINE;
      mtb.word.push(mw);
      return mtb;
    }

    let mut str = String::new();

    // the current link index, -1 if none
    let mut link_index: i32 = -1;

    let mut color = CursesColor::White;
    let mut use_char;
    let mut no_split_space;

    let i_max = chars.len();
    let mut i = 0;
    while i < i_max {
      let mut char_token = '\0';
      use_char = true;
      no_split_space = false;

      if chars[i] == ']' {
        // skip over ']'
        i += 1;
        if i >= i_max {
          break;
        }

        if chars[i] != ']' {
          // not "]]", check this char again from top in the next loop
          i -= 1;
          continue;
        }

        // else "]]", append ']' to str since use_char == true
      } else if chars[i] == '[' {
        // skip over '['
        i += 1;
        if i >= i_max {
          break;
        }

        if chars[i] == '.' || chars[i] == ':' || chars[i] == '?' || chars[i] == ' ' || chars[i] == '!' {
          // punctuation immediately after '['

          // use literal character
          no_split_space = true;
        } else if chars[i] != '[' {
          // not "[[", parse the markup token

          // do not use characters since it's a markup token
          use_char = false;

          // read until ':' or ']', and advance i accordingly
          let token_buffer = Self::grab_token_string_pos(&chars, i, ':');
          i += token_buffer.chars().count();

          // handle the token
          match token_buffer.as_str() {
            // Add a single character, either by code or directly
            // [CHAR:n]: takes a base-10 integer n representing a CP437 character
            // [CHAR:~ch]: accepts a character ch
            // No need to close the tag
            // Example: "[CHAR:154]" produces 'Ü', "[CHAR:~ ]" produces ' '
            "CHAR" => {
              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the argument
              let buff = Self::grab_token_string_pos(&chars, i, ':');
              let buff_chars = buff.chars().collect::<Vec<char>>();
              i += buff_chars.len();

              // parse the argument
              char_token = if buff_chars.len() > 1 && buff_chars[0] == '~' {
                // direct character starts with '~'
                buff_chars[1]
              } else {
                // otherwise parse as code, falling back to space on error
                char::from_u32(buff.parse::<u32>().unwrap_or(32)).unwrap_or(' ')
              };

              // use literal character
              no_split_space = true;

              // we finished processing this token, so use characters
              use_char = true;
            }
            // Start a MarkupLink. These are intended for Legends mode page links and don’t work in popups.
            // The text will just be colored based on LinkType.
            // Close the tag with [/LPAGE]
            "LPAGE" => {
              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the LinkType argument
              let buff_type = Self::grab_token_string_pos(&chars, i, ':');
              i += buff_type.len();

              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the id argument
              let buff_id = Self::grab_token_string_pos(&chars, i, ':');
              i += buff_id.len();

              // parse the LinkType
              let link_type = match buff_type.as_str() {
                "HF" => LinkType::HIST_FIG,
                "SITE" => LinkType::SITE,
                "ARTIFACT" => LinkType::ARTIFACT,
                "BOOK" => LinkType::BOOK,
                "SR" => LinkType::SUBREGION,
                "LF" => LinkType::FEATURE_LAYER,
                "ENT" => LinkType::ENTITY,
                "AB" => LinkType::ABSTRACT_BUILDING,
                "EPOP" => LinkType::ENTITY_POPULATION,
                "ART_IMAGE" => LinkType::ART_IMAGE,
                "ERA" => LinkType::ERA,
                "HEC" => LinkType::HEC,
                _ => LinkType::NONE,
              };

              // parse the link ID
              let id = buff_id.parse::<i32>().unwrap_or(0);
              let mut subid = -1;

              match link_type {
                // these link types have a subid argument, parse it
                LinkType::ABSTRACT_BUILDING | LinkType::ART_IMAGE => {
                  // skip over ':'
                  i += 1;
                  if i >= i_max {
                    break;
                  }

                  let buff_subid = Self::grab_token_string_pos(&chars, i, ':');
                  i += buff_subid.len();

                  subid = buff_subid.parse::<i32>().unwrap_or(0);
                }
                _ => {}
              }

              match link_type {
                LinkType::NONE => {}
                _ => {
                  // for link types that are recognized, create the link
                  let link = ManagedMarkupLink::new(link_type, id, subid);
                  mtb.link.push(link);

                  // set the current link index to the newly added link
                  link_index = mtb.link.len() as i32 - 1;
                }
              }
            }
            // End a MarkupLink.
            "/LPAGE" => {
              // commit the current string
              mtb.insert(&mut str, link_index, color);
              // reset link index
              link_index = -1;
            }
            // Color text. Sets the respective values in DF's gps global variable and then sets text color.
            // [C:screenf:screenb:screenbright]:
            //   screenf: integer color code for foreground (0-15)
            //   screenb: integer color code for background (0-15)
            //   screenbright: 1 for bright colors, 0 for normal colors
            // Note: screenb does nothing since popup backgrounds are always black.
            // Example: "Light gray, [C:4:0:0]red, [C:4:0:1]orange, [C:7:0:0]light gray."
            "C" => {
              // commit the current string
              mtb.insert(&mut str, link_index, color);

              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the first argument
              let buff1 = Self::grab_token_string_pos(&chars, i, ':');
              i += buff1.len();

              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the second argument
              let buff2 = Self::grab_token_string_pos(&chars, i, ':');
              i += buff2.len();

              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the third argument
              let buff3 = Self::grab_token_string_pos(&chars, i, ':');
              i += buff3.len();

              let mut local_screenf = 7;
              let mut local_screenbright = true;

              if buff1 == "VAR" {
                // color from dipscript var
                log::warn!(
                  "MTB_parse received:\n[C:VAR:{}:{}]\nwhich is for dipscripts and is unimplemented.",
                  buff2,
                  buff3
                );
                //MTB_set_color_on_var(mtb, buff2, buff3);
              } else {
                // skip setting colors in GPS, use local variables for colors
                local_screenf = buff1.parse::<i32>().unwrap_or(7);
                local_screenbright = buff3.parse::<i32>().unwrap_or(1) != 0;
              }

              // set color
              color = local_screenf.into();
              color = color.with_bright(local_screenbright);
            }
            // Keybinding. Shows the (first) keybinding for the interface_key.
            // The keybinding will be displayed in light green, but the previous text color will be restored afterwards
            // [KEY:n]: n is an integer interface_key code
            "KEY" => {
              // commit the current string
              mtb.insert(&mut str, link_index, color);

              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the argument
              let buff = Self::grab_token_string_pos(&chars, i, ':');
              i += buff.len();

              let mut ptr: ManagedMarkupWord = Default::default();
              let binding = buff.parse::<i32>().unwrap_or(0);

              // get the key display string from Dwarf Fortress, set color to light green, and append to mtb
              let key_ptr = cpp::CppString::new();
              df::enabler::get_key_display(key_ptr.raw(), binding);
              ptr.str = cp437_string::cxx_string_to_string(key_ptr.raw());
              ptr.color = df::gps::get_color_info().uccolor[CursesColor::LightGreen as usize];
              mtb.word.push(ptr);
            }
            // Deprecated dipscript variable insertion, does nothing
            // [VAR:format:type:name]
            "VAR" => {
              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the format argument
              let buff_format = Self::grab_token_string_pos(&chars, i, ':');
              i += buff_format.len();

              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the variable type argument
              let buff_var_type = Self::grab_token_string_pos(&chars, i, ':');
              i += buff_var_type.len();

              // skip over ':'
              i += 1;
              if i >= i_max {
                break;
              }

              // grab the variable name argument
              let buff_var_name = Self::grab_token_string_pos(&chars, i, ':');
              i += buff_var_name.len();

              // just log a debug message for now
              log::warn!(
                "MTB_parse received:\n[VAR:{}:{}:{}]\nwhich is for dipscripts and is unimplemented.",
                buff_format,
                buff_var_type,
                buff_var_name,
              );
            }
            // White space controls
            // [R]: New line, ends the current line and begins on the next.
            // [B]: Blank line, ends the current line and adds an additional blank line, beginning on the line after that.
            // [P]: Indent, ends the current line and begins four spaces indented on the next.
            "R" | "B" | "P" => {
              // commit the current string
              mtb.insert(&mut str, link_index, color);

              // create a empty MarkupWord with the respective flag
              let mut ptr: ManagedMarkupWord = Default::default();
              ptr.flags |= match token_buffer.as_str() {
                "R" => MARKUP_WORD_FLAG_NEWLINE,
                "B" => MARKUP_WORD_FLAG_BLANK_LINE,
                _ => MARKUP_WORD_FLAG_INDENT,
              };

              // push the MarkupWord to the word vector
              mtb.word.push(ptr);
            }
            // Ignored or unknown token, do nothing
            _ => {}
          }
        }
      }

      // if we are using characters, process the current character
      if use_char {
        // use CHAR token if specified, or use the current character
        let ch = if char_token == '\0' { chars[i] } else { char_token };

        // commit if the next character is CJK character
        if cjk::is_cjk(ch) && !cjk::is_cjk_punctuation(ch) {
          mtb.insert(&mut str, link_index, color);
        }

        // if split is not allowed
        if ch != ' ' || no_split_space {
          // commit the previous string if last character is CJK character
          if str.len() > 0 {
            let last_ch = str.chars().last().unwrap();
            if cjk::is_cjk(last_ch) && !cjk::is_cjk_punctuation(ch) {
              mtb.insert(&mut str, link_index, color);
            }
          }

          // append the character to the current string
          str.push(ch);
        } else {
          // commit the current string on space
          mtb.insert(&mut str, link_index, color);
        }
      }

      i += 1;
    }

    // commit any remaining string
    mtb.insert(&mut str, link_index, color);

    // post-process punctuation merging
    let mut i = mtb.word.len();
    while i > 1 {
      i -= 1;
      let (left, right) = mtb.word.split_at_mut(i);

      // skip if current entry is linked or empty
      let cur_entry = &right[0];
      if cur_entry.link_index != -1 || cur_entry.str.is_empty() {
        continue;
      }

      // skip if previous entry is not linked or empty
      let prev_entry = &mut left[i - 1];
      if prev_entry.link_index == -1 || prev_entry.str.is_empty() {
        continue;
      }

      // merge punctuation into previous word
      match cur_entry.str.chars().next().unwrap() {
        '.' | ',' | '?' | '!' | '。' | '，' | '？' | '！' => {
          prev_entry.str.push_str(&cur_entry.str);
          mtb.word.remove(i);
        }
        _ => {}
      }
    }

    // calculate and cache the width in pixels for each word
    for mw in &mut mtb.word {
      mw.width_px = mw.str.chars().map(|ch| glyph::get_glyph_orig_size(ch).0).sum();
    }

    // return the parsed MarkupTextBox
    mtb
  }

  // Original curses character width in pixels
  fn orig_width_in_pixels(&self) -> i32 {
    let orig_size = df::renderer::get_renderer_info().orig_size();
    orig_size.width
  }

  // Get current width in pixels
  fn current_width_in_pixels(&self) -> i32 {
    self.current_width * self.orig_width_in_pixels()
  }

  // Reset the width and max_y to start a new layout
  fn reset_text_block(&mut self, width: i32) {
    self.current_width = width;
    self.max_y = 0;
    self.text_block = text::TextBlock::from_columns(width as usize);
    self.current_row = text::TextRow::default();
    self.remain_width_px = self.current_width_in_pixels();
    self.occupied_x = -1;
    self.x_px = 0;
    self.y_val = 0;
  }

  // Insert a new line that move row cursor to the end of current row
  // by setting the remain_width_px to 0
  fn insert_newline(&mut self) {
    self.remain_width_px = 0;
  }

  // Push the current row to text block and reset row cursor to the beginning of next row
  fn push_current_row(&mut self) {
    // always attempt to remove the trailing space
    self.remove_last_space();

    // commit current row
    let current_row = mem::take(&mut self.current_row);
    self.text_block.push(current_row);

    // advance y position
    self.y_val += 1;

    // reset row cursor
    self.occupied_x = -1;
    self.x_px = 0;
    self.remain_width_px = self.current_width_in_pixels(); // TODO: this may overflow? should we minus 4?
  }

  // Insert a blank line
  fn insert_blank_line(&mut self) {
    self.push_current_row();
    self.insert_newline();
  }

  // Insert a text fragment to the current row
  fn insert_fragment(&mut self, fragment: text::TextFragment) {
    self.x_px += fragment.orig_width();
    self.remain_width_px -= fragment.orig_width();
    self.current_row.push(fragment);
  }

  //  Insert current word as a text fragment to the current row
  fn insert_current_word_fragment(&mut self, cur_word: ManagedMarkupWord) {
    let current_fragment = cur_word.into();
    self.word_x_start = self.x_px / self.orig_width_in_pixels();
    if self.word_x_start <= self.occupied_x {
      self.word_x_start = self.occupied_x + 1;
    }
    self.insert_fragment(current_fragment);
    self.word_x_end = (self.x_px - 1) / self.orig_width_in_pixels();
    if self.word_x_end < self.word_x_start {
      self.word_x_end = self.word_x_start;
    }
    self.occupied_x = self.word_x_end;
  }

  // Insert spaces to the current row
  fn insert_spaces(&mut self, count: usize) {
    let current_fragment = text::TextFragment::spaces(count);
    self.insert_fragment(current_fragment);
  }

  // Insert a single space
  fn insert_space(&mut self) {
    self.insert_spaces(1);
  }

  // Insert a newline then indent
  fn insert_indent(&mut self) {
    self.push_current_row();
    self.insert_spaces(4);
  }

  // Remove the last space in the current row, if any
  fn remove_last_space(&mut self) {
    if let Some(last_fragment) = self.current_row.last() {
      if last_fragment.content() == " " {
        self.x_px -= last_fragment.orig_width();
        self.remain_width_px += last_fragment.orig_width();
        self.current_row.pop();
      }
    }
  }

  // Update max_y when y_val is increased
  fn update_max_y(&mut self) {
    if self.max_y < self.y_val {
      self.max_y = self.y_val;
    }
  }

  // From dfhack/library/modules/Gui.cpp: Gui::MTB_set_width()
  pub fn set_width(&mut self, width: i32) {
    // early return for unchanged width
    if self.current_width == width {
      return;
    }

    // reset layout state
    self.reset_text_block(width);

    // for each word, decide which row to put it in
    for i in 0..self.word.len() {
      let cur_word = self.word[i].clone();

      // ends the current line and begins on the next.
      if cur_word.flags & MARKUP_WORD_FLAG_NEWLINE != 0 {
        self.insert_newline();
        continue;
      }

      // ends the current line and adds an additional blank line, beginning on the line after that.
      if cur_word.flags & MARKUP_WORD_FLAG_BLANK_LINE != 0 {
        self.insert_blank_line();
        continue;
      }

      // ends the current line and begins four spaces indented on the next.
      if cur_word.flags & MARKUP_WORD_FLAG_INDENT != 0 {
        self.insert_indent();
        continue;
      }

      // we need warp if the current word doesn't fit in the remaining width
      if self.remain_width_px < cur_word.width_px {
        self.push_current_row();
      }

      // if the next word is a single punctuation character that cannot fit to the current line, move current word to the next line
      if i + 1 < self.word.len() {
        let next_word = &self.word[i + 1];
        if next_word.str.chars().count() == 1 {
          let next_char = next_word.str.chars().next().unwrap();
          if self.x_px > 0 && self.remain_width_px < cur_word.width_px + self.orig_width_in_pixels() * 2 {
            match next_char {
              '.' | ',' | '?' | '!' => {
                self.push_current_row();
              }
              _ => {}
            }
          }
        }
      }

      // if current word is a single punctuation character, move it to the end of the previous word
      if cur_word.str.chars().count() == 1 && self.x_px > 0 {
        let cur_char = cur_word.str.chars().next().unwrap();
        match cur_char {
          '.' | ',' | '?' | '!' => {
            // insert the current word fragment to the current row
            // and update its x, y position
            self.insert_current_word_fragment(cur_word.clone());
            self.word[i].x = self.word_x_start;
            self.word[i].y = self.y_val;
            self.word[i].width = self.word_x_end - self.word_x_start + 1;

            // update max_y
            self.update_max_y();

            continue;
          }
          _ => {}
        }
      }

      // insert the current word fragment to the current row
      // and update its x, y position
      self.insert_current_word_fragment(cur_word.clone());
      self.word[i].x = self.word_x_start;
      self.word[i].y = self.y_val;
      self.word[i].width = self.word_x_end - self.word_x_start + 1;

      // update max_y
      self.update_max_y();

      // add a space after the word
      self.insert_space();

      // if both current and next word are CJK characters, remove the space between them
      if i + 1 < self.word.len() {
        let next_word = &self.word[i + 1];
        if cur_word.str.chars().count() > 0 && next_word.str.chars().count() > 0 {
          let cur_last_char = cur_word.str.chars().last().unwrap();
          let next_first_char = next_word.str.chars().next().unwrap();
          if cjk::is_cjk(cur_last_char) && cjk::is_cjk(next_first_char) {
            // remove the space just added
            self.remove_last_space();
          }
        }
      }
    }

    // commit the current row
    self.push_current_row();
  }

  // Sync the ManagedMarkupTextBox to the in-game MarkupTextBox at address
  pub fn sync_mtb(&self, address: usize, fill: bool) {
    let mtb = unsafe { (address as *mut df::game::MarkupTextBox).as_mut_unchecked() };
    let mut word: cpp::CppVector<df::game::MarkupTextWord> = cpp::CppVector::from_raw(ptr::from_mut(&mut mtb.word));
    word.clear();
    for mw in &self.word {
      let cpp_string = if fill {
        // use 'z' as placeholder for markup texts, and mark with ID later
        cpp::CppString::fill(b'z', mw.width as usize)
      } else {
        cpp::CppString::from_slice(&cp437_string::string_to_cp437_codepoints(&mw.str))
      };
      hooks::call_mtb_process_string_to_lines(address as *const ffi::c_void, cpp_string.raw());
      let mtw = word.get(word.size() - 1);
      mtw.link_index = mw.link_index;
      mtw.px = mw.x;
      mtw.py = mw.y;
      mtw.flags = mw.flags;
    }
    mtb.current_width = self.current_width;
    mtb.max_y = self.max_y;
    // TODO: sync links as well?
  }

  // Get the text block
  pub fn text_block(&self) -> text::TextBlock {
    self.text_block.clone()
  }

  // Grab a token string from source starting at pos until compc or ']'
  // From g_src/basics.cpp: grab_token_string_pos()
  // TODO: move to utils?
  fn grab_token_string_pos(source: &Vec<char>, pos: usize, compc: char) -> String {
    let mut out = String::new();

    // go until you hit a compc, or the end
    for i in pos..source.len() {
      if source[i] == compc || source[i] == ']' {
        break;
      }
      out.push(source[i]);
    }

    out
  }

  // Insert the current str as a MarkupWord into the word vector
  // From dfhack/library/modules/Gui.cpp: insert_markup_text_wordst()
  fn insert(&mut self, str: &mut String, link_index: i32, color: CursesColor) -> bool {
    if str.is_empty() {
      return false;
    }

    // create a new MarkupWord
    let mut ptr: ManagedMarkupWord = Default::default();
    ptr.str = str.clone();
    ptr.link_index = link_index;
    ptr.color = df::gps::get_color_info().uccolor[color as usize];

    // push the MarkupWord to the word vector and clear the input string
    self.word.push(ptr);
    str.clear();

    return true;
  }
}

// Get or parse a MarkupTextBox by its markup string
pub fn get(markup: &str) -> ManagedMarkupTextBox {
  let mut markups = get_markups_mut();

  if !markups.contains_key(markup) {
    let mtb = ManagedMarkupTextBox::parse(markup);
    markups.insert(markup.to_string(), mtb);
  }

  markups.get_mut(markup).unwrap().clone()
}

// Set width and sync the MarkupTextBox at address
pub fn set_width_and_sync(markup: &str, width: i32, address: usize, fill: bool) {
  let mut markups = get_markups_mut();

  if let Some(mtb) = markups.get_mut(markup) {
    mtb.set_width(width);
    mtb.sync_mtb(address, fill);
  }
}

// Sync the MarkupTextBox at address
pub fn sync(markup: &str, address: usize, fill: bool) {
  let mut markups = get_markups_mut();
  if let Some(mtb) = markups.get_mut(markup) {
    mtb.sync_mtb(address, fill);
  }
}

// Track the original markup of a MarkupTextBox by its address
pub fn track_mtb_markup(address: usize, markup: String) {
  let mut mtbs = get_mtbs_mut();
  mtbs.insert(address, markup);
}

// Fetch the original markup of a MarkupTextBox by its address
pub fn fetch_mtb_markup(address: usize) -> Option<String> {
  let mtbs = get_mtbs();
  mtbs.get(&address).cloned()
}
