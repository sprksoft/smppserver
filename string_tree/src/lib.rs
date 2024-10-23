mod wordlist;

#[derive(Debug, PartialEq)]
pub enum StringTree<T> {
    Node {
        substring: Box<str>,
        next: Vec<StringTree<T>>,
    },
    Leaf(Box<str>, T),
}
impl<T> StringTree<T> {
    pub fn from_iter(iter: impl Iterator<Item = (impl Into<Box<str>>, T)>) -> Self {
        let mut root_node = StringTree::Node {
            substring: "".into(),
            next: Vec::new(),
        };
        for (string, item) in iter {
            let boxx = string.into();
            match root_node.partial_search_node(&boxx, &boxx) {
                Some(node) => match node {
                    Self::Leaf(str, item) => {}
                    _ => {}
                },
                None => {}
            }
        }

        root_node
    }

    pub fn add(&mut self, new_item: (Box<str>, T)) {
        match self {
            Self::Node { substring, next }
            _=>panic!("Can't add to leaf node"),
        }
    }

    pub fn partial_search_node(
        &self,
        search_str: &str,
        searchsub_str: &str,
    ) -> Option<&StringTree<T>> {
        match self {
            Self::Node { substring, next } => {
                if searchsub_str.starts_with(substring.as_ref()) {
                    let new_search_str = &searchsub_str[substring.len()..];
                    for child in next.iter() {
                        match child.partial_search_node(search_str, new_search_str) {
                            Some(node) => return Some(node),
                            None => {}
                        }
                    }
                    Some(self)
                } else {
                    None
                }
            }
            Self::Leaf(string, _) => {
                if string.as_ref() == search_str {
                    Some(self)
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_test() {
        let wordlist = "apple\nable\nability\nboy\nbox\ncall";
        let tree = StringTree::from_iter(wordlist.lines().map(|s| (s, ())));
        let manual = StringTree::Node {
            substring: "".into(),
            next: vec![
                StringTree::Node {
                    substring: "a".into(),
                    next: vec![
                        StringTree::Leaf("apple".into(), ()),
                        StringTree::Node {
                            substring: "b".into(),
                            next: vec![
                                StringTree::Leaf("able".into(), ()),
                                StringTree::Leaf("ability".into(), ()),
                            ],
                        },
                    ],
                },
                StringTree::Node {
                    substring: "bo".into(),
                    next: vec![
                        StringTree::Leaf("boy".into(), ()),
                        StringTree::Leaf("box".into(), ()),
                    ],
                },
                StringTree::Leaf("call".into(), ()),
            ],
        };

        assert_eq!(tree, manual);
    }

    #[test]
    fn search() {
        let tree = StringTree::from_iter(wordlist::LIST.lines().map(|s| (s, ())));

        assert_eq!(
            tree.partial_search_node("who", "who"),
            Some(&StringTree::Leaf("who".into(), ()))
        );
    }
}
