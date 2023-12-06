mod utils;
mod dfa;
mod nfa;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, wasm-fa!");
}

#[wasm_bindgen]
pub fn get_ans(input: &str) -> String {
    re_to_dfa(input)
}

fn re_to_dfa(re: &str) -> String {
    let nfa = nfa::Builder::new().build_nfa_from_re(&re.to_string()).unwrap();
    let non_epsilon_nfa = nfa::Builder::new().build_non_epsilon_nfa(&nfa).unwrap();
    let new_dfa = dfa::DFA01::build_dfa_from_nfa(&non_epsilon_nfa);
    let newnew_dfa = dfa::DenseDFA::build_from_sparse01_dfa(&new_dfa);

    if let Some(minimized) = newnew_dfa.minimize() {
        minimized.to_string()
    } else {
        newnew_dfa.to_string()
    }
}