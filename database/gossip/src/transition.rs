use std::collections::HashSet;
use std::collections::VecDeque;
use std::iter::FromIterator;
use ring::digest;

use PeerId;

///
/// Inspector for incoming transitions. 
///
/// This trait has to be implemented for anyone using this library. It 
/// checks an unknown translation and gives functions to store such 
/// translations and retrieve it later from a database.
pub trait Inspector {
    fn approve(&self, trans: &Transition) -> bool;
    fn store(&self, trans: Transition);
    fn restore(&self, keys: Vec<TransitionKey>) -> Vec<Transition>;
    fn tips(&self) -> Vec<TransitionKey>;
    fn has(&self, key: &TransitionKey) -> bool;

    fn subgraph(&self, mut tips: Vec<Transition>) -> Vec<Transition> {
        let mut in_transitions: HashSet<Transition> = HashSet::from_iter(tips.iter().cloned());

        while tips.len() < 64 {
            tips = tips.into_iter()
                .map(|x| self.restore(x.refs)).flatten().collect();

            for tip in &tips {
                in_transitions.insert(tip.clone());
            }

            if tips.is_empty() {
                break;
            }
        }
        
        let mut tips = self.restore(self.tips());
        let mut queue = VecDeque::from_iter(tips.iter().cloned());

        while !queue.is_empty() {
            let a = match queue.pop_front() {
                Some(x) => {
                    tips.push(x.clone());

                    x
                },
                None => break
            };

            for b in self.restore(a.refs) {
                if !in_transitions.contains(&b) {
                    queue.push_back(b);
                }
            }
        }

        tips.into_iter().collect()
    }
}

/// Transition key is the 256bit hash of the body
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransitionKey(pub [u8; 32]);

impl TransitionKey {
    pub fn from_vec(buf: &[u8]) -> TransitionKey {
        let mut key = TransitionKey([0; 32]);

        key.0.copy_from_slice(&buf);

        key
    }

    pub fn to_string(&self) -> String {
        let mut tmp = String::new();
        for i in 0..16 {
            tmp.push_str(&format!("{:02x}", (self.0)[i]));
        }

        tmp
    }

}
/// A signed transition in a DAG
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transition {
    pub pk: PeerId,
    pub refs: Vec<TransitionKey>,
    pub body: Option<Vec<u8>>,
    pub sign: [u8; 32],
    pub is_tip: bool
}

impl Transition {
    /// Ignore signature for now
    pub fn new(pk: PeerId, refs: Vec<TransitionKey>, data: Vec<u8>) -> Transition {

        Transition {
            pk,
            refs,
            body: Some(data),
            sign: [0; 32],
            is_tip: true
        }
    }

    pub fn key(&self) -> TransitionKey {
        let mut key = TransitionKey([0u8; 32]);

        // build buffer from refs and body
        let mut buf = Vec::new();
        for a in &self.refs {
            buf.extend_from_slice(&a.0);
        }

        buf.extend_from_slice(&self.body.clone().unwrap());

        let hash = digest::digest(&digest::SHA256, &buf);
        key.0.copy_from_slice(&hash.as_ref());

        key
    }
}
