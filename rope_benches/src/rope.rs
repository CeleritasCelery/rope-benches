// use std::ops::RangeBounds;

// #[derive(Debug)]
// pub enum RopeError {
//     PositionOutOfBounds,
// }

pub trait Rope: From<String> {
    const NAME: &'static str;
    const EDITS_USE_BYTE_OFFSETS: bool = false;

    fn new() -> Self;

    fn insert_at(&mut self, pos: usize, contents: &str);// -> Result<(), RopeError>;
    fn del_at(&mut self, pos: usize, len: usize);// -> Result<(), RopeError>;
    fn edit_at(&mut self, pos: usize, del_len: usize, ins_content: &str) {
        if del_len > 0 {
            self.del_at(pos, del_len);
        }
        if !ins_content.is_empty() {
            self.insert_at(pos, ins_content);
        }
    }
    fn to_string(&self) -> String;
    fn char_len(&self) -> usize;
    fn line_search(&self, re: &regex::Regex) -> usize;
    fn full_search(&self, re: &regex::Regex) -> usize {
        let string = self.to_string();
        re.find(string.as_str()).map(|m| m.start()).unwrap_or_else(|| self.char_len())
    }
}
