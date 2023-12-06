/*! 
本项目是正则语言的各种表示形式之间的转换的实现，
适用于完成《形式语言与自动机》课程的实验作业。
 
包含这些功能：
- 正则表达式 -> 带空转移的NFA
- 带空转移的NFA -> 不带空转移的NFA
- 不带空转移的NFA -> DFA
- DFA的极小化
- DFA -> 正则文法
- DFA的状态转移表的格式化打印

# 用法

阅读本文档就能了解这个包的结构。

点击每个页面右上角的 `source` 即可观看源码。

[在网页上使用编译好的程序](http://60.204.241.149:8002/)。

克隆这个项目的[仓库]()，使用 `cargo run` 即可在命令行中使用。

使用 `wasm-pack build` 可以将本项目编译为WebAssembly模块，以供网页调用。
*/
mod utils;
pub mod dfa;
pub mod nfa;

use wasm_bindgen::prelude::*;

/// 输入正则表达式，返回对应的DFA的状态转移表和对应的正则文法。
#[wasm_bindgen]
pub fn get_ans(input: &str) -> String {
    let dfa = re_to_dfa(input);
    let ans = dfa.to_string();
    let rg = dfa.to_rg();
    format!("{}@{}", ans, rg)
}

/// 将正则表达式转化为极小化DFA。
pub fn re_to_dfa(re: &str) -> dfa::DenseDFA {
    let nfa = nfa::Builder::new().build_nfa_from_re(&re.to_string()).unwrap();
    let non_epsilon_nfa = nfa::Builder::new().build_non_epsilon_nfa(&nfa).unwrap();
    let new_dfa = dfa::DFA01::build_dfa_from_nfa(&non_epsilon_nfa);
    let newnew_dfa = dfa::DenseDFA::build_from_sparse01_dfa(&new_dfa);

    if let Some(minimized) = newnew_dfa.minimize() {
        minimized
    } else {
        newnew_dfa
    }
}