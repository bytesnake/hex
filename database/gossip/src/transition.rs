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
    fn restore(&self, keys: Vec<TransitionKey>) -> Option<Vec<Transition>>;
    fn tips(&self) -> Vec<TransitionKey>;
    fn has(&self, key: &TransitionKey) -> bool;
    fn get_file(&self, key: &[u8]) -> Option<Vec<u8>>;

    fn subgraph(&self, mut tips: Vec<Transition>) -> Vec<Transition> {
        //println!("Got tips {}", tips.clone().into_iter().map(|x| x.key.to_string()).collect::<Vec<String>>().join(","));

        // create a sample of the subgraph, starting by the given tips
        let mut in_transitions: HashSet<Transition> = HashSet::from_iter(tips.iter().cloned());

        while in_transitions.len() < 64 {
            tips = tips.into_iter()
                .map(|x| {
                    let refs = x.refs.into_iter().filter(|x| self.has(&x)).collect();

                    self.restore(refs).unwrap()
                }).flatten().collect();

            for tip in &tips {
                in_transitions.insert(tip.clone());
            }

            if tips.is_empty() {
                break;
            }
        }
        
        trace!("Got {} transitions for checking", in_transitions.len());

        //trace!("My tips are {:?}", self.tips().into_iter().map(|x| x.to_string()));

        // start at our tips and run till we reach the sampled transitions
        let tips = self.restore(self.tips()).unwrap();
        let mut queue = VecDeque::from_iter(tips.iter().cloned());
        let mut transitions = Vec::new();

        while !queue.is_empty() {
            let a = match queue.pop_front() {
                Some(x) => {
                    if !in_transitions.contains(&x) {
                        trace!("Transition {}", x.key.to_string());
                        transitions.push(x.clone());
                    }

                    x
                },
                None => break
            };


            let refs = a.refs.into_iter().filter(|x| self.has(&x)).collect();
            for b in self.restore(refs).unwrap() {
                if !in_transitions.contains(&b) {
                    queue.push_back(b);
                }
            }
        }

        trace!("Returning {} transitions", transitions.len());

        transitions
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
        for i in 0..32 {
            tmp.push_str(&format!("{:02X}", (self.0)[i]));
        }

        tmp
    }

}
/// A signed transition in a DAG
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transition {
    pub key: TransitionKey,
    pub pk: PeerId,
    pub refs: Vec<TransitionKey>,
    pub body: Option<Vec<u8>>,
    pub sign: [u8; 32],
    pub state: u8
}

impl Transition {
    /// Ignore signature for now
    pub fn new(pk: PeerId, refs: Vec<TransitionKey>, data: Vec<u8>) -> Transition {

        let mut tmp = Transition {
            key: TransitionKey([0u8; 32]),
            pk,
            refs,
            body: Some(data),
            sign: [0; 32],
            state: 2
        };

        tmp.key = tmp.key();

        tmp
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
