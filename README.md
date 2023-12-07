## 说明

这是一个《形式语言与自动机》课程的实验作业项目，内容是正则语言的各种表示形式之间的转换。

## 调试或编译

这是一个rust项目，所以你需要安装 rust 工具链。

[按照这些说明安装 rust 工具链](https://www.rust-lang.org/zh-CN/tools/install) 。

### 本地调试

在 scr 目录新建一个 main.rs 文件，写入：
```rust
use wasm-fa::{dfa, nfa};

fn main() {
  // 在这里写调试代码
}
```
即可在 main.rs 内写调试的代码。

运行 `cargo doc --open` 来查看api文档。

### 编译为 WebAssembly 模块

先安装用于构建、测试和发布rust生成的WebAssembly的集成工具 [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) 。

然后运行：

```
wasm-pack build
```
即可在pkg目录找到构建完成的 WebAssembly 模块。

www 目录是一个使用这个模块的网页的示例。阅读这个目录内的 readme.md 可找到如何启动这个网页。

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
