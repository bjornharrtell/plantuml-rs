//! Longest-match dictionary of names (`Trie`).

use std::collections::HashMap;

#[derive(Default, Clone)]
struct Node {
    children: HashMap<char, usize>,
    terminal: bool,
}

#[derive(Clone)]
pub struct Trie {
    nodes: Vec<Node>,
}

impl Default for Trie {
    fn default() -> Self {
        Self {
            nodes: vec![Node::default()],
        }
    }
}

impl Trie {
    pub fn add(&mut self, s: &str) {
        let mut cur = 0;
        for c in s.chars() {
            cur = match self.nodes[cur].children.get(&c) {
                Some(&n) => n,
                None => {
                    self.nodes.push(Node::default());
                    let n = self.nodes.len() - 1;
                    self.nodes[cur].children.insert(c, n);
                    n
                }
            };
        }
        self.nodes[cur].terminal = true;
    }

    pub fn remove(&mut self, s: &str) -> bool {
        let mut cur = 0;
        for c in s.chars() {
            match self.nodes[cur].children.get(&c) {
                Some(&n) => cur = n,
                None => return false,
            }
        }
        std::mem::replace(&mut self.nodes[cur].terminal, false)
    }

    /// Name found in `s` at char index `pos`, descending as far as possible; empty unless
    /// the node reached is the end of a name (`getLonguestMatchStartingIn`).
    pub fn longest_match_starting_in(&self, s: &[char], pos: usize) -> String {
        let mut cur = 0;
        let mut i = pos;
        while i < s.len() {
            let Some(&child) = self.nodes[cur].children.get(&s[i]) else {
                break;
            };
            if self.nodes[child].children.is_empty() && !self.nodes[child].terminal {
                break;
            }
            cur = child;
            i += 1;
        }
        if self.nodes[cur].terminal {
            s[pos..i].iter().collect()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_match() {
        let mut t = Trie::default();
        t.add("a");
        t.add("abc");
        let s: Vec<char> = "abcd".chars().collect();
        assert_eq!(t.longest_match_starting_in(&s, 0), "abc");
        let s: Vec<char> = "abd".chars().collect();
        assert_eq!(t.longest_match_starting_in(&s, 0), "");
        let s: Vec<char> = "azz".chars().collect();
        assert_eq!(t.longest_match_starting_in(&s, 0), "a");
        assert!(t.remove("abc") && !t.remove("zz"));
        let s: Vec<char> = "abc".chars().collect();
        assert_eq!(t.longest_match_starting_in(&s, 0), "");
    }
}
