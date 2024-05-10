// #[cfg(test)]

// use crate::*;

// #[test]
// fn test_read_brackets() {
//     let raw_line = "  [CREATURE:DWARF] optional comment text ".to_string();
//     let (brackets, line_vec, comments) = Graphics::read_brackets(&raw_line);
    
//     assert_eq!(true, brackets);
//     assert_eq!("CREATURE", line_vec[0]);
//     assert_eq!("DWARF", line_vec[1]);
//     assert_eq!(2, line_vec.len());
//     assert_eq!("optional comment text", comments.unwrap());
// }
