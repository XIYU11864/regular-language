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

[尝试一下](http://60.204.241.149:8002/)。

查看这个项目的[仓库](https://github.com/XIYU11864/regular-language)。
*/
mod utils;

/// DFA 相关的结构体和方法。
/// 
/// 本模块包含了DFA的结构体和方法，以及从NFA构建DFA的方法。
/// 
/// 使用u128类型来表示状态id，原因是从NFA构造DFA的方法是幂集构造法，
/// 理论上如果原始NFA的状态数为n，那么构造出来的DFA的状态数为2^n。
/// 因此需要尽量大的数作为状态id。u128是rust中最大的整数类型。
/// 
/// 这个模块包含了两个用于表示DFA的模型，稀疏DFA和稠密DFA。
/// 
/// 它们的区别是储存状态转移函数的位置。稀疏DFA定义了一个表示状态的结构体，并把从这个状态出发的状态转移函数储存在这个结构体中。
/// 而稠密DFA把所有状态转移函数统一储存在一个Vec中。
/// 
/// 这两种模型的表达能力是相同的，只不过稀疏DFA的空间效率更高，适合进行“写”操作，而稠密DFA的时间效率更高，适合“读”操作。
pub mod dfa;

/// NFA 相关的结构体和方法。
/// 
/// 本模块包含了NFA的结构体和方法，以及从正则表达式构建NFA的方法。
pub mod nfa;

use wasm_bindgen::prelude::*;

/// 输入正则表达式，返回对应的DFA的状态转移表和对应的正则文法。
#[wasm_bindgen]
pub fn get_ans(input: &str) -> String {
    let dfa = re_to_dfa(input);
    let ans = dfa.to_string();
    let rg = dfa.to_rg();
    let dot = dfa.call_to_dot();
    format!("{}@{}@{}", ans, rg, dot)
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