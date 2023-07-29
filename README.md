# 简述
* 这是一个编译器, 它将`mindustry_logic_bang`语言编译到`Mindustry`游戏中的`逻辑语言`
* `逻辑语言`是游戏`Mindustry`中一种名为`逻辑处理器`的建筑使用的语言, 这里指其序列化后的文本
* `mindustry_logic_bang`语言提供了如`goto` `if` `while` `switch`等语句, 使得编写`逻辑语言`更加方便
* `逻辑语言`中, 主要的跳转方式就是`goto`.
  `mindustry_logic_bang`语言主要目的就是组织代码的嵌套结构
  并编译到`逻辑语言`, 使得基于`逻辑语言`的项目开发更为容易
* 这个语言是零开销的, 也就是说你不用付出任何的中间代价, 它足够快
* 通过一种叫`DExp`表达式的值, 可以多语句计算并返回值, 放在循环条件还有一些地方很好用
* 这个语言对于一些特殊的语句进行了处理, 例如对`print`会将每个参数分别展开成一行`print`,
  来避免输出信息写七八行`print`的窘境<br/>
  再例如`op`, 我们既可以用原版风格写法`op add a a 1`, 也可以使用变换后的写法`op a a add 1`,
  并且这里对`op`和`jump`的比较都进行了符号化, 比如上面的可以写成`op + a a 1` `op a a + 1`,<br/>
  条件则可以用`skip n < 2 { print ">="; }`, 它等价于`skip lessThan n 2 { print ">="; }`
* 通过`DExp`配合内联常量`const`可以完成类似宏的定义与调用, 这可以提供很好的代码复用
* 提供了很多示例, 在`examples/`中, 可以通过里面的示例来学习这门语言
* 为`vim` `MT 管理器`提供了基础的语法(高亮)文件
* 速度: 因为是rust写的, 并使用了`lalrpop`框架, 编译几千几万行代码也是几乎瞬间完成, 内存占用可忽略
* 项目编译占用: 因为使用了大型框架`lalrpop`, 会被生成约二十万行代码,
  编译需要相对较长时间, 并且占用1-2G磁盘空间
