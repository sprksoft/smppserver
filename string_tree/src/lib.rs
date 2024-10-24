use std::fmt::Debug;

mod wordlist;

#[derive(PartialEq)]
struct StringTree {
    children: Vec<StringTree>,
    str: Box<str>,
    leaf: bool,
}
impl StringTree {
    pub fn new_node(str: &str, children: Vec<StringTree>) -> Self {
        Self {
            children,
            str: str.into(),
            leaf: false,
        }
    }
    pub fn new_leaf(str: impl Into<Box<str>>) -> Self {
        Self {
            children: vec![],
            str: str.into(),
            leaf: true,
        }
    }
}
impl Debug for StringTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.str)?;
        if self.children.len() != 0 {
            if self.leaf {
                f.write_str("[ERROR]")?;
            }
            f.write_str(":{ ")?;
            for child in self.children.iter() {
                child.fmt(f)?;
                f.write_str(" ")?;
            }

            f.write_str("}")?;
        }
        Ok(())
    }
}

impl StringTree {
    pub fn from_iter(iter: impl Iterator<Item = impl Into<Box<str>>>) -> Self {
        let mut root_node = Self::new_node("", vec![]);
        for item in iter {
            let into = item.into();
            root_node.add(into, 0);
        }

        root_node
    }

    fn common_str<'a>(str1: &'a str, str2: &str) -> &'a str {
        for (i, (byte1, byte2)) in str1.bytes().zip(str2.bytes()).enumerate() {
            if byte1 != byte2 {
                return &str1[..i];
            }
        }
        str1
    }

    fn split_leaf(&mut self, mut common_str: Box<str>) {
        self.leaf = false;
        std::mem::swap(&mut common_str, &mut self.str);
        self.children.push(Self::new_leaf(common_str));
    }

    pub fn add(&mut self, new_item: Box<str>, min_common: usize) -> bool {
        let common = Self::common_str(self.str.as_ref(), new_item.as_ref());
        if self.str.len() == 0 || common.len() > min_common {
            for child in self.children.iter_mut() {
                if child.add(new_item.clone(), common.len()) {
                    return true;
                }
            }
            if self.leaf {
                self.split_leaf(common.into());
            } else {
                if common.len() < self.str.len() {
                    //TODO: split node
                }
            }
            self.children.push(Self::new_leaf(new_item));
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_str() {
        assert_eq!(StringTree::common_str("apple", "able"), "a");
        assert_eq!(StringTree::common_str("able", "apple"), "a");
        assert_eq!(StringTree::common_str("box", "boy"), "bo");
        assert_eq!(StringTree::common_str("boy", "box"), "bo");
        assert_eq!(StringTree::common_str("bo", "box"), "bo");
        assert_eq!(StringTree::common_str("cow", "through"), "");
    }

    #[test]
    fn build_test() {
        let wordlist = "able\nability\nboy\nbox\ncall\napple";
        let tree = StringTree::from_iter(wordlist.lines());
        let manual = StringTree::new_node(
            "",
            vec![
                StringTree::new_node(
                    "a".into(),
                    vec![
                        StringTree::new_leaf("apple"),
                        StringTree::new_node(
                            "ab",
                            vec![
                                StringTree::new_leaf("able"),
                                StringTree::new_leaf("ability"),
                            ],
                        ),
                    ],
                ),
                StringTree::new_node(
                    "bo",
                    vec![StringTree::new_leaf("boy"), StringTree::new_leaf("box")],
                ),
                StringTree::new_leaf("call"),
            ],
        );

        assert_eq!(tree, manual);
    }

    // #[test]
    // fn search() {
    //     let tree = StringTree::from_iter(wordlist::LIST.lines());
    //
    //     assert_eq!(
    //         tree.partial_search_node("who", "who"),
    //         Some(&StringTree::new_leaf("who"))
    //     );
    // }
}
