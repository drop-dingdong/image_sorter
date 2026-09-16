pub mod idx_name {
    use once_cell::sync::Lazy;
    use regex::Regex;
    #[derive(Debug, Default)]
    pub enum RenamePatternState {
        Iter(IndexName),
        Err(String),
        #[default]
        Empty,
    }
    impl RenamePatternState {
        pub fn is_iter(&self) -> bool {
            match self {
                RenamePatternState::Iter(_) => true,
                _ => false,
            }
        }
    }
    // 用于迭代生成按序排泄的图片的名称，
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct IndexName {
        pub style: (u32, u32, u32), // 字母数目， 数字上限，数字位数
        pub max_size: u32,          // 当前style最大允许的排列的图片个数
        pub now_num: u32,           // 当前文件重命名所需的
        pub current_index: (Vec<char>, u32),
    }
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^a(\d+)b(\d+)$").unwrap());

    fn next_vec_char(chars: &mut Vec<char>) {
        // chars中所有的char均在'a' - 'z'之间
        let len = chars.len();
        let mut flag = 1;
        for i in (0..len).rev() {
            if flag == 0 {
                break;
            }
            let mut chari = chars[i];
            if chari == 'z' {
                flag = 1;
                chari = 'a';
            } else {
                flag = 0;
                chari = ((chars[i] as u8) + 1) as char;
            }
            chars[i] = chari;
        }
    }
    impl IndexName {
        pub fn new(style: &String) -> Option<Self> {
            let cap = RE.captures(style.as_str())?;
            let alphabet_num: u32 = (cap.get(1)?).as_str().parse::<u32>().ok()?;
            let numeric_num: u32 = (cap.get(2)?).as_str().parse::<u32>().ok()?;
            let mut vec = Vec::with_capacity(alphabet_num as usize);
            for _ in 0..(alphabet_num as usize) {
                vec.push('a');
            }
            let max_num = (10 as u32).pow(numeric_num) - 1;
            Some(IndexName {
                style: (alphabet_num, max_num, numeric_num),
                max_size: (26 as u32).pow(alphabet_num) * (max_num + 1),
                now_num: 0,
                current_index: (vec, 0),
            })
        }
    }

    impl Iterator for IndexName {
        type Item = String;
        fn next(&mut self) -> Option<String> {
            if self.now_num >= self.max_size {
                return None;
            }
            let bigit_num = self.style.2;
            let index_string = if bigit_num != 0 {
                format!(
                    "{}{:0>width$}",
                    (self.current_index.0).iter().collect::<String>(),
                    self.current_index.1,
                    width = self.style.2 as usize
                )
            } else {
                format!("{}", (self.current_index.0).iter().collect::<String>(),)
            };
            if self.current_index.1 == self.style.1 {
                self.current_index.1 = 0;
                next_vec_char(&mut self.current_index.0);
            } else {
                self.current_index.1 += 1;
            }
            self.now_num += 1;
            Some(index_string)
        }
    }
}
#[cfg(test)]
mod text_idx_name {
    use crate::idx_name;
    #[test]
    fn text_idx_name() {
        let str0 = "a2".to_string();
        let idx_na = idx_name::IndexName::new(&str0);
        assert_eq!(idx_na, None);
        let str1 = "".to_string();
        let idx_na = idx_name::IndexName::new(&str1);
        assert_eq!(idx_na, None);
        let str2 = "ab".to_string();
        assert_eq!(None, idx_name::IndexName::new(&str2));
        let str3 = "a0b1".to_string();
        let idx_na = idx_name::IndexName::new(&str3);
        assert_eq!(
            idx_na,
            Some(idx_name::IndexName {
                style: (0, 9, 1),
                max_size: (10),
                now_num: 0,
                current_index: (vec![], 0)
            })
        );
        let mut idx_na = idx_na.unwrap();
        let idx_na_point = &mut idx_na;
        for i in 0..10 {
            assert_eq!(Some(format!("{}", i)), idx_na_point.next());
        }
        assert_eq!(None, idx_na_point.next());
        let str3 = "a1b0".to_string();
        let idx_na = idx_name::IndexName::new(&str3);
        assert_eq!(
            idx_na,
            Some(idx_name::IndexName {
                style: (1, 0, 0),
                max_size: (26),
                now_num: 0,
                current_index: (vec!['a'], 0)
            })
        );
        let mut idx_na = idx_na.unwrap();
        let idx_na_point = &mut idx_na;
        assert_eq!(Some("a".to_string()), idx_na_point.next());
        assert_eq!(Some("b".to_string()), idx_na_point.next());
        assert_eq!(Some("c".to_string()), idx_na_point.next());
        for _ in 0..22 {
            idx_na_point.next();
        }
        assert_eq!(Some("z".to_string()), idx_na_point.next());
        assert_eq!(None, idx_na_point.next());
        let str3 = "a1b2".to_string();
        let idx_na = idx_name::IndexName::new(&str3);
        assert_eq!(
            idx_na,
            Some(idx_name::IndexName {
                style: (1, 99, 2),
                max_size: 26 * 100,
                now_num: 0,
                current_index: (vec!['a'], 0),
            })
        );
        let mut idx_na = idx_na.unwrap();
        let idx_na_point = &mut idx_na;
        assert_eq!(Some("a00".to_string()), idx_na_point.next());
        assert_eq!(Some("a01".to_string()), idx_na_point.next());
        for i in 2..(idx_na_point.max_size - 1) {
            idx_na_point.next();
        }
        assert_eq!(Some("z99".to_string()), idx_na_point.next());
        assert_eq!(idx_na_point.now_num, idx_na_point.max_size);
        assert_eq!(idx_na_point.next(), None);

        let str4 = "a2b1".to_string();
        let idx_na = idx_name::IndexName::new(&str4);
        assert_eq!(
            idx_na,
            Some(idx_name::IndexName {
                style: (2, 9, 1),
                max_size: 26 * 26 * 10,
                now_num: 0,
                current_index: (vec!['a', 'a'], 0),
            })
        );
        let mut idx_na = idx_na.unwrap();
        let idx_na_point = &mut idx_na;
        assert_eq!(Some("aa0".to_string()), idx_na_point.next());
        assert_eq!(Some("aa1".to_string()), idx_na_point.next());
        for i in 2..(idx_na_point.max_size - 1) {
            idx_na_point.next();
        }
        assert_eq!(Some("zz9".to_string()), idx_na_point.next());
        assert_eq!(idx_na_point.now_num, idx_na_point.max_size);
        assert_eq!(idx_na_point.next(), None);
    }
}
