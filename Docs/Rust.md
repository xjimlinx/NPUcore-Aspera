---
title: Rust学习笔记
categories: 
    - 编程语言
    - Rust
tags:
    - Rust
    - 编程
    - 基础
date: 2024-10-9 15:50:57
author: Xein
comments: true
description: Rust学习笔记
---
# Rust学习笔记

![dancing-ferris](Rust/dancing-ferris.gif "Ferris——Rust社区吉祥物" )

## 〇、基础入门

### 0.基础语法

#### 0.1 变量

> Rust 语言具有可变变量和不可变变量的概念
>
> 可变变量：Mutable 即在后续过程中可以改变该变量数值字面量
>
> 不可变变量：Immutable 与上述相反
>
> 且 Rust 是强类型语言，具有自动判断变量类型的能力
>
> 为什么要这样做？
>
> Rust 声称为了高并发安全而做的设计 ~~我现在也不懂，等以后再回来看看~~

```rust
let a = 123; // 不可变变量
let mut b = 123; // 可变变量
a = 12; // 此句会报错
b = 1; // 此句不会报错
b = "changed"; // 此句会报错 因为上面的声明中 b 明显为 int 类型
b = 12.3; // 此句会报错 因为12.3 由 float 转换为 int 类型精度会有损失，而Rust不允许该情况发生（精度由损失的自动数据类型转换的情况）
```

可以看到，只要在声明关键字 **let** 后面添加 关键字 **mut** 就可以使 **变量** 成为 **可变变量**

#### 0.1.1 变量解构

```rust
let (a, mut b): (bool, bool) = (true, true);
```

#### 0.2 常量

```rust
const a: i32 = 123;
let a = 456;	// 此处不合法，因为a此前为常量
```

#### 0.3 显示声明

```rust
let a: u64 = 123;	// 此处a将会被判断为 u64 类型
let a = 123;		// 此处a将会被判断为 i32 类型
```

#### 0.4 注释

```rust
// 单行注释 MK1

/* 单行注释 MK2 */

/*
	多行注释
*/

/// 说明文档的注释
/// 用的是 markdown 语法
```

---

### 1.数据类型

#### 1.1 基本类型

| 类型关键字 | 类型含义                  | 类型范围 | 类型例子 |
| ---------- | ------------------------- | -------- | -------- |
| i8         | 8位integer                |          |          |
| i16        | 16位integer               |          |          |
| i32        | 32位integer               |          | 12       |
| i64        | 64位integer               |          | -23      |
| i128       | 128位integer              |          | -1231231 |
| isize      | 与CPU位数相同             |          |          |
| u8         | 8位unsigned               |          | 12       |
| u16        | 16位unsigned              |          | 32       |
| u32        | 32位unsigned              |          | 3213     |
| u64        | 64位unsigned              |          | 3213131  |
| u128       | 128位unsigned             |          |          |
| usize      | 与CPU位数相同             |          |          |
| f32        | 32位float                 |          |          |
| f64        | 64位float（默认使用这种） |          |          |
| bool       | 布尔类型                  |          | true     |
| char       | 字符                      |          | 'R'      |

注意char 使用单引号

String类型使用双引号

#### 1.2 复合类型

> 可以使用编译器属性标记 "#[allow(unused_variables)]" 来让编译器忽略未使用的变量

##### 1.2.1 元组

> 元组可以包含不同类型数据

例子：

```rust
let tup: (i32, u8, f32) = (-100, 100, 1.0);
// tup.0 = -100
// tup.1 = 100
// tup.2 = 1.0
let (x, y, z) = tup;
// x = -100
// y = 100
// z = 1.0
```

##### 1.2.2 数组

> 数组只能包含同类型数据

例子：

```rust
let a = [1, 2, 3, 4, 5];

let b = ["apple", "banana", "orange"];

let c: [i32; 5] = [1, 2, 3, 4, 5];
// 5 表示长度，i32 表示类型

let d = [3; 5];
// 即 let d = [3, 3, 3, 3, 3];

let first = a[0];
let second = a[1];
// 数组的访问

a[0] = 123; // 数组不可变，所以报错
let mut a = [1, 2, 3];
a[0] = 4; // 此处不会报错
```

##### 1.2.3 字符串

分为静态字符串 &str 和 动态字符串 String

其中 &str 不可变， String 可变

**&str 的大小固定为（因为相当于是一个指针或者说引用）2个字长（即2个CPU位数），在64位CPU上即为16B，包含一个指针字段（包含地址）和字符串长度字段（包含该字符串长度）的数据**

**切片（Slice）**

对于字符串而言，切片就是对String类型的某一部分的引用

```rust
let s = String::from("Hello world");

let hello = &s[0..5];
let world = &s[6..11];
```

hello 没有引用整个 String s，而是引用了 s 的一部分内容，通过[0..5] 来指定

创建切片的语法，使用方括号包括的一个序列：[开始索引..终止索引]，其中开始索引是切片中第一个元素的索引位置，而终止索引是最后一个元素后一个的索引位置。长度为 终止索引 - 开始索引。

在rust中

[0..2] 与 [..2] 等效

同样的，假设长度为len，

[3..len] 与 [3..] 等效

所以完整切片

[0..len] 与 [..] 等效

> 字符串切片的类型标识是&str

> 除了字符串之外，其他集合类型也有，如数组

> &str 是一个不可变引用！！！

**String 与 &str切换**

```rust
String::from("xxx");
"xxx".to_string();

// String取引用即是&str类型
let s = String::from("xxx");
// &s;
// &s[..];
// s.as_str();
```

**字符串操作**

+ 追加（ Push）

```rust
fn main() {
    let mut s = String::from("Hello ");
  
    s.push_str("rust");
    println!("追加字符串 push_str() -> {}", s);
  
    s.push("!");
    println!("追加字符 push() -> {}", s);
}
```

+ 插入（Insert）
+ 替换（Replace） 返回一个新的字符串（三种操作方式）
+ 删除（Delete）pop() 删除并返回最后一个字符，remove() 删除并返回字符串中指定位置的字符

    truncate() 删除字符串中从指定位置开始到结尾的全部字符，无返回值

    clear() 清空字符串

+ 连接（Concatenate）返回一个新的字符串
+ 可使用 format! 来连接字符串，功能与c语言下的fprintf类似

**字符串转义**

```rust
// \ 字符功能与其他语言类似
```

使用如下方法可以直接保持字符串原样：

```rust
let raw_str = r"hahah\dw\a\d\213\fd\as \gb\fd\u{211D}";

// 若字符串包含双引号，则可以在开头和结尾添加 #
let quotes_raw = r#"djiasoidjaosi"dqdwjihui""#;

// 若字符串包含#号，则可以在开头和结尾多加个#号，最多255个，只需要保证与字符串中连续#号个数不超过开头和结尾的#号个数
```

**操作UTF-8字符串**

+ 使用字符方式遍历字符串

```rust
for c in "中国人".chars() {
	println!("{}", c);
}

// output
中
国
人
```

+ 使用字节方式

```rust
for b in "中国人".bytes() {
    println!("{}", b);
}

// output 将会返回字符串的底层字节数组表现形式
228
184
173
229
155
189
228
186
186
```

+ 获取子串

std下的方法做不到，可以使用 utf8_slice crate

#### 1.3 运算

##### 1.3.1 基本运算

无非加减乘除模

##### 1.3.2 位运算

| 运算符 | 含义                                         |
| ------ | -------------------------------------------- |
| &      | 位与                                         |
| \|     | 位或                                         |
| ^      | 位异或                                       |
| !      | 非                                           |
| <<     | 左移指定位数，右位补0                        |
| >>     | 右移指定位数，带符号移动（正数补0，负数补1） |

#### 1.4 序列（Range）

```rust
for i in 1..=5 {
    println!("{}", i);
}
Output:
1
2
3
4
5

for i in 1..5 {
    println!("{}", i);
}
Output:
1
2
3
4
```

> 序列只允许数字或者字符类型

#### 1.5 类型转换

```rust
value1 as type2
```

#### 1.6 复数

使用num库

例子：

```toml
# Cargo.toml
# 添加以下内容
[dependencies]
num = "0.4.0"


# main.rs
use num::complex::Complex
```

#### 1.7 单元类型

() 即是一个单元类型，大小为0

非发散函数(diverge function 为发散函数)再没有显示返回一个表达式的情况下，会返回一个单元类型

控制块也是！如下：

```rust
let x = 5;
let z = {
    2*x;
}
// 则此时z = ()，是一个单元类型
```

#### 1.8 发散函数

用 ! 作函数返回类型的时候，表示该函数永不返回（ diverge function ），特别的，这种语法往往用做会导致程序崩溃的函数：

```rust
fn dead_end() -> ! {
    panic!("Panic!!!");
}
```

下面函数创建了一个无限循环，该循环永不跳出，因此函数也永不返回：

```rust
fn forever() -> !{
    loop {
        //...
    };
}
```

可以使用如下四种方式实现发散函数（即永远不返回）

```rust
panic!();
todo!(); // not yet implemented 将导致panic
unimplemented!(); // not implemented 将导致panic
loop{};
```

---

### 2.函数

基本形式：

```rust
fn <函数名> ( [参数列表] ) [-> <返回值>] {<函数体>}
```

更为具体的例子：

```rust
fn function_name (arg_1: type1, arg_2: type2, ... arg_n: typen) -> ret_type {
    function body
}
```

其中 “**-> ret_type**“ 在没有返回值的情况下可以省略，此时默认返回类型为 **()** ，即**空元组**

> 语句：执行某些操作且没有返回值的步骤。如：
>
> let a = 1;
>
> 表达式：有计算步骤且有返回值。如：
>
> a = 1
>
> b + 1
>
> a + b * c

#### 函数体表达式

```rust
{
    let a = 1;
    a + 1
};
// 该表达式块最后一个步骤是表达式，该表达式结果值即为整个表达式块所代表的值，这种表达式块叫做函数体表达式
```

#### 函数返回值

```rust
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}// 如果没有明确声明返回值类型，不可以在 return 后面添加表达式！

fn add1(a: i32, b: i32)-> i32 {
    a + b;
}// 与上面一样
```

---

### 3.流程控制

#### 3.1 if-else

```rust
fn main() {
    let n = 3;
    if n < 5 {
		println!("True");
    } else {
        println!("False");
    }
  
    if n == 3 {
        println!("n == 3");
    }
  
    if n == 2 {
        println!("n == 2");
    } else if n == 1 {
        println!("n == 1");
    } else {
        println!("n != 2 && n != 1");
    }
}
// 其中 if 后面的以及 else if 后面的表达式必须是bool类型，不像部分其他的编程语言有非0即真的特性
```

##### 3.1.1 三目运算符的效果

```rust
let number = if a > 0 {1} else { -1 };
```

#### 3.2 loop

> 当做没有条件判断的 **while** 来使用

```rust
fn main() {
    // 如果没有break,那么就是个死循环
	let mut count = 0;
	loop {
        count += 1;
        println!("Hello World! {} times!", count);
        if count >= 100 {
            break;
        }
    }
}
```

#### 3.3 while

> 可以将 **while** 视为带有条件判断的 **loop**

```rust
 fn main(){
     let mut number = 1;
     while number < 4 {
         println!("{}", number);
         number += 1;
     }
     println!("EXIT");
}
```

> rust 将do 设置为保留关键字，也就是将来还会用到

#### 3.4 for

> for 会创建一个迭代器
>
> 其中已经实现copy特征的数据类型可以省略掉iter()方法，因为不需要引用
>
> 而未实现的数据类型需要使用iter()方法，这样会创建一个引用，不会转移所有权
>
> iter()的enumerate()方法会返回一个元组，构成为(i, v)，i为索引，v为数值

---

### 4.其他复合类型

#### 4.1 结构体

##### ⚠TODO！

#### 4.2 切片

##### ⚠TODO！

#### 4.3 枚举

##### ⚠TODO！

---

### 5.所有权

> 计算机语言不断演变过程中，出现了三种流派：
>
> + 垃圾回收机制 GC： 在程序运行时不断寻找不再使用的内存，典型：Java、Go
> + 手动管理内存的分配和释放：在程序中通过函数调用方式来申请和释放内存，典型：C++
> + 通过所有权来管理内存：编译器在编译时会根据一系列规则进行检查
>
> Rust选择了第三种，这种检查只会在编译期出现，对于程序运行期不会有任何性能上的损失

**所有权原则**：

+ Rust中每一个值都被一个变量所拥有，该变量被称为值的所有者
+ 一个值同时只能被一个变量所拥有，或者说一个值只能拥有一个所有者
+ 当所有者（变量）离开作用域范围时，这个值将被丢弃（drop）

> 当拷贝过于简单时（固定大小的简单值），不会发生所有权转移（发生的是**自动拷贝**），比如：

```rust
let a = 1;
let b = a;
// b 和 a 在不同内存上，没有发生所有权转移
```

#### 5.1 Move 移动（转移所有权）

对于基本类型（存储在栈上的），Rust 会自动拷贝，但是String不是基本类型，存储在堆上，所以不能自动拷贝

> String 由存储在栈上的**堆指针**、**字符串长度**、**字符串容量**共同组成
>
> String 类型指向了一个堆上的空间

**二次释放**

```rust
let s1 = String::from("hello");
let s2 = s1;
```

此处若是深拷贝，由于是在堆上的数据，全部拷贝会对性能产生非常大的影响（当数据量大时）

若是浅拷贝，即只拷贝String本身，则这个值出现了两个所有者，而 Rust 的机制，当变量离开作用域时，会自动调用drop函数并清理变量的堆内存，不过由于两个String变量指向了同一位置。那么当s1 和s2 离开作用域时，都会尝试释放相同的内存。即**二次释放（double free）**错误。属于内存安全性bug之一，两次释放相同内存会导致内存污染，可能会导致潜在的安全漏洞。

因此，Rust 会在s1赋予给s2之后，认为s1不再有效，无需再在s1离开作用域时drop任何东西，即把所有权从s1转移给了s2。

其他语言中有术语**浅拷贝（shallow copy）**和**深拷贝（deep copy）**，拷贝指针、长度和容量而不拷贝数据就像是浅拷贝，但是又因为Rust同时使第一个变量s1无效了，因此这个操作称之为**移动（move）**，而不是浅拷贝，可以理解为s1被移动到s2.

#### 5.2 Clone 克隆（深拷贝）

Rust 永远也不会自动创建数据的”深拷贝“。因此任何自动的复制都不是深拷贝，可以被认为对运行时性能影响较小。

如果确实需要深拷贝，可以使用clone方法

```rust
let s1 = String::from("hello");
let s2 = s1.clone();

println!("s1 = {}, s2 = {}", s1, s2);
```

#### 5.3 Copy 复制（浅拷贝）

浅拷贝只会发生在栈上，因此性能很高

```rust
let x = 5
let y = x;
println!("x = {}, y = {}", x, y);
```

像整型这样的基本类型在编译时是已知大小的，会被存储在栈上，所以拷贝其实际的值是快速的。（可以理解成此处发生了在栈上做了深拷贝）

**任何基本类型的组合可以copy，不需要分配内存或者某种形式资源的类型是可以copy的。**如：

+ 所有整数类型
+ 布尔类型
+ 所有浮点数类型
+ 字符类型
+ 元组
+ 不可变引用 &T （注意 可变引用 &mut T 是不可以copy的）

#### 5.4 函数传值与返回

将值传递给函数，一样会发生移动或者复制，就跟let语句一样，如下展示所有权、作用域的规则：

```rust
fn main() {
	let s = String::from("hello");	// s 进入作用域
  
    takes_ownership(s);				// s 的值移动到函数内
    								// 故 s在此之后不再有效
    let x = 5;						// x 进入作用域
  
    makes_copy(x);					// x应该移动到函数里，但是i32是可copy的，所以在后面可以继续用x
  
    println!("{}", s);				// 该句会报错
    println!("{}", x);				// 该句不会报错
}

fn takes_ownership(some_string: String) {	// some_string 进入作用域
	println!("{}", some_string);
}											// some_string 移出作用域并调用`drop`方法，占用的内存释放

fn makes_copy(some_integer: i32) {			// some_integer 进入作用域
    println!("{}", some_integer);
}											// some_integer移出作用域，不会有特殊操作
```

同样，函数返回值也有所有权

```rust
fn main() {
	let s1 = gives_ownership();					// gives_ownership 将返回值
    											// 移给s1
    let s2 = String::from("hello");				// s2 进入作用域
  
    let s3 = takes_and_gives_back(s2);			// s2 被移动到takes_and_gives_back 中
    											// 它也将返回值移给s3
}	// 此处，s3移出作用域被丢弃，s1移出作用域被丢弃，s2已被移动不会发生丢弃。

fn gives_ownership() -> String {				// gives_ownership 将返回值移动给
    											// 调用它的函数
	let some_string = String::from("hello");	// some_string 进入作用域
    some_string									// 返回some_string 并移出给调用的函数
}

// takes_and_gives_back 将传入字符串并返回该值
fn takes_and_gives_back(a_string:String) -> String{	// a_string 进入作用域
    a_string										// 返回 a_string 并移出给调用的函数
}
```

#### 5.5 引用（借用） Borrowing

Rust 只允许同时存在一个可变引用或多个不可变引用

如下：

```rust
struct Foo {
    x: i32
}

fn do_something(a: &Foo) -> &i32 {
    return &a.x;
}

fn main() {
    let mut foo = Foo { x: 42 };
    let x= &mut foo.x;
    // foo borrowed as mutable
    *x = 13;
    println!("{}", x);
    // 上面这一行加上去不会导致报错，因为在这里过后
    // x就被释放了

    let y = do_something(&foo);
    println!("{}", y);
    // println!("{}", x); // 此处被注释
    // 如果没有被注释，那么x 未被释放，foo仍存在一个可变引用
    // 因此不能再出现上面的不可变引用，所以会导致do something函数报错
}
```

##### 5.5.1 引用与解引用

```rust
fn main() {
    let x = 5;
    let y = &x;			// 引用
  
    assert_eq!(5, x);
    assert_eq!(5, *y);	// 解引用
}
```

可以通过引用的方式来解决释放问题，如下：

```rust
fn do_something(f: &mut i32){
    // 此处需要先解引用
    *f += 1;
    // 此步过后f被释放
}

fn main(){
    let mut foo = 2i32;
    println!("{}", foo);
    do_something(&mut foo);
    println!("{}", foo);
}
```

输出为

```bash
2
3
```

##### 5.5.2 不可变引用

```rust
fn main() {
    let s1 = String::from("hello");
  
    let len = calculate_length(&s1);
  
    println!("The length of '{}' is {}.", s1, len);
}

fn calculate_length(s: &String) -> usize {
    s.len()
}
```

此处 &符号即是引用，允许使用值而不是所有权。

**但是无法通过这个不可变引用来修改引用指向的值！**

##### 5.5.3 可变引用

```rust
fn main() {
	let mut s = String::from("hello");
  
    change(&mut s);
}

fn change(some_string: &mut String) {
	some_string.push_str(", world");
}
```

> 可变引用同时只能存在一个！！！

> 可变引用与不可变引用不能同时存在！！!

**NLL**

Non-Lexical-Lifetimes(NLL)

Ruts的编译器优化行为，专门用于找到某个引用在作用域（}）结束前就不再被使用的代码位置

##### 5.5.4 悬垂引用 Dangling References

也叫做悬垂指针

```rust
fn main() {
    let reference_to_nothing = dangle();
}

fn dangle() -> &String {
    let s = String::from("hello");
  
    &s
} // s在这里被丢弃，&s将不指向任何值
```

上述代码会报错（返回了一个悬垂指针）

将返回值类型改为String, &s改为s可以解决

---

### 6.模式匹配

#### 6.1 match

```rust
fn main() {
	let x = 5;
    let y = match x{
        0..2 => 5;
        3 => 7;
        4|8|9 => 9;
        _ => 10;
    }
} 
```

#### 6.2 matches! 宏

##### ⚠TODO！

#### 6.3 if let

> 当模式匹配仅要匹配一个模式其他模式不管的时候使用 if let

```rust
fn main() {
	let x = Some(5);
    // if let 后面可以理解为一个判断真值的表达式
    if let Some(5) = x{
        println!("x == Some(5)");
	}
}
```

#### 6.4 匹配守卫

```rust
// 形式如下
fn main() {
    let x = 8;
    let y = true;
    match x {
        1..9 if y => println!("yes"),
        _ => println!("no"),
    }
}
// [if ... ]部分就是匹配守卫
```

#### 6.2 @绑定

```rust
enum Message {
    Hello { id: i32 },
}

let msg = Message::Hello { id: 5 };
fn main() {
	match msg {
    	Message::Hello { id: id_variable @ 3..=7 } => {
        	println!("Found an id in range: {}", id_variable)
    	}, // 此处id_variable 可以绑定 id 中的值
    	Message::Hello { id: 10..=12 } => {
        	println!("Found an id in another range")
    	}, // 此处无法使用id中的值
    	Message::Hello { id } => {
        	println!("Found some other id: {}", id)
    	},
	}
}
```

##### 6.2.1 @其他注意事项

> 在使用@绑定包含 **"|"** 的情况时，需要将后面的匹配的模式加上括号，否则会当作只匹配第一个
>
> 如下第12行

```rust
enum Message {
    Hello { id: i32 },
}

fn main() {
    let msg = Message::Hello { id: 5 };

    match msg {
        Message::Hello {
            id:  id@ 3..=7,
        } => println!("id 值的范围在 [3, 7] 之间: {}", id),
        Message::Hello { id: newid@ (10 | 11 | 12) } => {
            println!("id 值的范围在 [10, 12] 之间: {}", newid)
        }
        Message::Hello { id } => println!("Found some other id: {}", id),
    }
}
```

---

### 7. 方法Method

> 与面向对象语言中的方法差不多

#### 7.1 定义方法

```rust
struct Square {
    x: f64
}

impl Square {
    // new 是 Square 的关联函数，因为它的第一个参数不是self,且new并不是关键字
    // 这种方法往往用于初始化当前结构体的实例
    fn new(x: f64) -> Square {
        Square {
            x: x,
        }
    }
  
    // Square 的方法，&self 表示借用当前的Square 结构体
    fn area(&self) -> f64 {
        self.x * self.x
    }
}
```

Rust语言与其他语言的对比

![img](https://pica.zhimg.com/80/v2-0d848e960f3279999eab4b1317f6538e_1440w.png)

> 这样将对象和方法定义分离是为了给予使用者更高的灵活度（数据和使用分离的方式）

如上代码中

```rust
impl Square {}
// 表示impl语句块内的一切都和Square相关联
```

注意，当rust在调用方法时，会自动引用与解引用，即自动添加&、&mut、或者* 来使代码与方法签名匹配

> 方法和函数一样可以使用多个参数

#### 7.2 关联函数

> 方法的参数中不包含self的称之为关联函数

如构造函数就可以使用关联函数

> 同一个结构体可以构造多个 impl 块，以提供更多灵活性和代码组织性

**枚举也可以使用方法！！！**

> 可以使用Self用于在构造函数内表示当前结构体类型

```rust
struct Square{
    x: u32
}

impl Square {
    pub fn new() -> Self{
        Self{
            x:1,
        }
    }
}
```

---

### 8.泛型和特征 Generics & Traits

#### 8.1 泛型 Generics

> 有点像 **C++** 里的函数模板

```rust
fn add<T: std::ops::Add<Output = T>>(a:T, b:T) -> T {
    a + b
}

fn main() {
    println!("add i8: {}", add(2i8, 3i8));
    println!("add i32: {}", add(20, 30));
    println!("add f64: {}", add(1.23, 1.23));
}
```

> 出于惯例，常使用T（type的首字母）作为泛型参数的首选名称

泛型在使用之前，需要先进行声明，如下：

```rust
fn largest<T>(list: &[T]) -> T {...}
// 如上largest<T>对泛型参数进行了声明，然后才在函数参数中进行使用该泛型参数 list: &[T]
```

##### 8.1.1 泛型参数约束

```rust
use std::fmt::Display;

fn create_and_print<T>() where T: From<i32> + Display {
    let a: T = 100.into(); // 创建了类型为 T 的变量 a，它的初始值由 100 转换而来
    println!("a is: {}", a);
}

fn main() {
    create_and_print();
}

// 此处的where T: From<i32> + Display 指定了 T 类型必须实现From<i32> 以及 Display 的特征
```

**结构体和枚举以及方法也可以使用泛型**（泛型参数可以有多个）

> 可以使用如下方法进行多个泛型参数的使用

```rust
struct Point<T, U> {
    x:T,
    y:U,
}
fn main() {
	let p = Point{x: 1, y: 1.1};
}
```

> 此外注意尽量不要让泛型的参数个数和代码复杂度过高！！

**可以为具体类型定义方法**

```rust
impl Point<f32> {
    fn distance_from_origin(&self) -> f32 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
}
```

##### 8.1.2 const 泛型

```rust
fn display_array<T: std::fmt::Debug, const N: usize>(arr: [T; N]) {
    println!("{:?}", arr);
}
fn main() {
    let arr: [i32; 3] = [1, 2, 3];
    display_array(arr);

    let arr: [i32; 2] = [1, 2];
    display_array(arr);
}
// 上面的display_array函数为泛型 N 指定了const,且基于的值的类型是usize
```

> 有了const泛型之后，Rust将变得适合复杂矩阵的运算

###### 8.1.2.1 const 泛型表达式

目前只能在nightly版本内使用，先跳过

###### 8.1.2.2 const fn

> const fn 用于编译期间就需要知道计算结果的场景

```rust
const fn add(a: usize, b: usize) -> usize {
    a + b
}

const RESULT: usize = add(5, 10);

fn main() {
    println!("The result is: {}", RESULT);
}
```

**Rust 中的泛型是零成本抽象，即使用泛型时，完全不用担心性能上的问题，但是编译的时候，Rust 会对代码进行单态化（monomorphization）来提升运行效率，就好像手写了每个具体定义的重复代码，所以没有运行时开销，但是编译速度会变慢，最终生成文件大小会变大**

#### 8.2 特征 Trait

> 如果不同的类型具有相同行为，那么就可以定义一个特征，接着为这些类型实现该特征
>
> 定义特征是把一些方法组合在一起，目的是定义一个实现某些目标所必需的行为的集合

特征只定义行为看起来是怎么样而不去具体实现。

因此只定义特征方法的签名（即方法名、参数、以及返回值等），而不进行实现，此时方法签名结尾是;

而不是{}

(即类似c中的只声明)

如下例子Summary为Weibo和Post实现了特征

```rust
// 此处声明特征 Summary
pub trait Summary {
    fn summarize(&self) -> String;
}

// 此处声明结构体struct
pub struct Post {
    pub title: String, // 标题
    pub author: String, // 作者
    pub content: String, // 内容
}

// 此处为 Post 实现特征 Summary
impl Summary for Post {
    fn summarize(&self) -> String {
        format!("文章{}, 作者是{}", self.title, self.author)
    }
}

// 此处声明结构体 weibo
pub struct Weibo {
    pub username: String,
    pub content: String
}

// 此处为 weibo 实现特征 Summary
impl Summary for Weibo {
    fn summarize(&self) -> String {
        format!("{}发表了微博{}", self.username, self.content)
    }
}
```

> 实现特征的语法与为结构体、枚举实现方法很像：`impl Summary for Post`，即为 Post 实现 Summary 特征

接下来就可以在类型上调用特征的方法：

```rust
fn main(){
	let post = Post{title: "Rust语言简介".to_string(), author:"Sunface".to_string(), content:"Rust屌爆了！".to_string()};
    let weibo = Weibo{username: "sunface".to_string(), content: "疑似有点太极端了".to_string()};
  
    println!("{}", post.summarize());
    println!("{}", weibo.summarize());
}
```

##### 8.3.1 特征定义与实现的位置（孤儿规则）

如果想要为类型A实现特征T,则A或者T至少有一个要在当前作用域被定义

##### 8.3.2 默认实现

```rust
pub trait Echo {
    fn echo(&self) -> String {
        println!("Hello from default implementation!");
    }
}

impl Echo for Weibo {
    fn echo(&self) {
        println!("Hello from Weibo!")
    }
}
```

**特征也可以作为函数参数**

```rust
pub fn notify(item: &impl Summary) {
    println!("Breaking news! {}", item.summarize());
}
```

##### 8.3.3 特征约束（trait bound）

形如下方为参数添加了Echo的特征约束（即参数必须要实现Echo特征）

```rust
pub fn notify<T: Echo> (item1: &T, item2: &T) {}
```

`T: Echo` 说明T必须要实现Echo特征，而内部的参数说明 `item1`和 `item2`必须有相同类型

###### 8.3.3.1多重约束

形如下方为参数添加了多个特征约束，称为多重约束

```rust
pub fn notify<T: Summary + Display>(item: &T) {}
```

也可以用如下形式

```rust
pub fn notiry(item: &(impl Summary + Display)) {}
```

###### 8.3.3.2 Where 约束

当参数和特征约束变得很多时，可以使用where来使代码更易读，也可以保留原有格式：

```rust
fn some_func<T: Display + Clone, U: Clone + Debug>(t: &T, u: &U) -> i32{}
```

使用where作改进之后：

```rust
fn some_func<T, U>(t: &T, u: &U) -> i32
	where T: Display + Clone,
		  U: Clone + Debug
{}
```

**可以通过函数返回 impl Trait**

```rust
fn returns_summarizable() -> impl Summary {
    Weibo {
        username: String::from("sunface"),
        content: String::from(
         "这是内容",
        )
    }
}
```

如上因为Weibo实现了Summary特征，所以可以用它来作为返回值

但是这种情况下只能返回一种类型！（除非使用特征对象）

**通过derive派生特征**

在结构体前加上形如 `#[derive(Debug)]`的标记，就可以为其自动实现对应的Debug特征代码。

derive提供的是rust默认提供的特征，如有特殊需求，可以手动重载实现

#### 8.3 特征对象 `Todo!`

> 在拥有继承的语言中，可以定义一个名为 `Component` 的类，该类上有一个 `draw` 方法。其他的类比如 `Button`、`Image` 和 `SelectBox` 会从 `Component` 派生并因此继承 `draw` 方法。它们各自都可以覆盖 `draw` 方法来定义自己的行为，但是框架会把所有这些类型当作是 `Component` 的实例，并在其上调用 `draw`。不过 Rust 并没有继承，我们得另寻出路。

为了解决上面的所有问题，Rust引入了一个概念——特征对象

```rust
// 定义特征
trait Draw {
    fn draw(&self) -> String;
}

impl Draw for u8 {
    fn draw(&self) -> String {
        format!("u8: {}", *self)
    }
}

impl Draw for f64 {
    fn draw(&self) -> String {
        format!("f64: {}", *self)
    }
}

// 若 T 实现了 Draw 特征， 则调用该函数时传入的 Box<T> 可以被隐式转换成函数参数签名中的 Box<dyn Draw>
fn draw1(x: Box<dyn Draw>) {
    // 由于实现了 Deref 特征，Box 智能指针会自动解引用为它所包裹的值，然后调用该值对应的类型上定义的 `draw` 方法
    x.draw();
}

fn draw2(x: &dyn Draw) {
    x.draw();
}

pub struct Screen {
    pub components: Vec<Box<dyn Draw>>,
}

impl Screen {
    pub fn run(&self) {
        for component in self.components.iter() {
            component.draw();
        }
    }
}

fn main() {
    let x = 1.1f64;
    // do_something(&x);
    let y = 8u8;

    // x 和 y 的类型 T 都实现了 `Draw` 特征，因为 Box<T> 可以在函数调用时隐式地被转换为特征对象 Box<dyn Draw> 
    // 基于 x 的值创建一个 Box<f64> 类型的智能指针，指针指向的数据被放置在了堆上
    draw1(Box::new(x));
    // 基于 y 的值创建一个 Box<u8> 类型的智能指针
    draw1(Box::new(y));
    draw2(&x);
    draw2(&y);
}
```

如上代码中可以发现，`draw1`和 `draw2`使用了不同的参数，

+ `draw1`的参数是 `Box<dyn Draw>`形式的特征对象，该特征对象通过 `Box::new(x)`创建
+ `draw2`的参数是 `&dyn Draw`形式的特征对象，该特征对象通过 `&x`的方式创建
+ `dyn`关键字只用在特征对象的类型声明上，在创建时无需使用 `dyn`

**使用特征对象作为函数返回值**

```rust
fn hatch_a_bird(num: u32) -> Box<dyn Bird> {
    match num {
        1 => Box::new(Swan{}),
        2 => Box::new(Duck{}),
        _ => Box::new(Swan{}),
    }  
}
// 如上函数返回了具有Bird特征的返回值
// 注意这里的Swan和Duck将会丢失Bird特征内部未实现的方法
```

##### 8.3.1 特征对象的动态分发

```rust
trait Bird {
    fn quack(&self);
}

struct Duck;
impl Duck {
    fn fly(&self) {
        println!("Look, the duck is flying")
    }
}
struct Swan;
impl Swan {
    fn fly(&self) {
        println!("Look, the duck.. oh sorry, the swan is flying")
    }
}

impl Bird for Duck {
    fn quack(&self) {
        println!("{}", "duck duck");
    }
}

impl Bird for Swan {
    fn quack(&self) {
        println!("{}", "swan swan");
    }
}

fn main() {
    // 此处以数组形式来使用特征对象
    let birds: [Box<dyn Bird>; 2] = [Box::new(Duck {}), Box::new(Swan {})];

    for bird in birds {
        bird.quack();
        // 当 duck 和 swan 变成 bird 后，它们都忘了如何翱翔于天际，只记得该怎么叫唤了。。
        // 因此，以下代码会报错
        // bird.fly();
    }
}
```

如上当duck和swan在使用Bird特征对象后，会丢失原有的未在Bird特征内实现的特性。

这是因为特征对象的动态分发导致的，后面再来仔细看看这个特性

###### ⚠TODO！

##### 8.3.2 Self 与 self

> Rust中有两个self,一个指代当前的实例对象，一个指代特征或者方法类型的别名

```rust
trait Draw{
    fn draw(&self) -> Self;
}
```

上述代码中：

+ Self：特征（当作为impl 代码块中的Self时，一般指返回的类型）
+ self：当前实例对象

##### 8.3.3 特征对象的限制

只有对象安全的特征才可以拥有特征对象，当一个特征的所有方法都有如下属性时，它的对象才是安全的：

+ 方法的返回类型不能是Self
+ 方法没有任何泛型参数

#### 8.4 进一步深入

##### 8.4.1 关联类型

> 在特征定义的语句块中，申明一个自定义类型，这样就可以在特征的方法签名中使用该类型，这个就是该特征的关联类型

```rust
pub trait Iterator {
	type Item;
  
    fn next(&mut self) -> Option<Self::Item>;
}
```

以上就是标准库中的迭代器特征 `Iterator`,有一个 `Item`关联类型，用于替代遍历的值的类型。

##### 8.4.2 默认泛型类型参数

> 当使用泛型类型参数时，可以为其指定一个默认的具体类型，例如标准库中的std::ops::Add特征：

```rust
trait Add<RHS=Self> {
    type Output;
  
    fn add(self, rhs: RHS) -> Self::Output;
}
```

可以看到有一个泛型参数 `RHS`, 但是这里给 `RHS`一个默认值，也就是当用户不指定 `RHS`时，默认使用两个同样类型的值进行相加，然后返回一个关联类型 `Output`

###### 8.4.2.1 运算符重载

```rust
use std::ops::Add;

#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

fn main() {
    assert_eq!(Point { x: 1, y: 0 } + Point { x: 2, y: 3 },
               Point { x: 3, y: 3 });
}
```

如上代码为 `Point`类型重载了（实现了）`+`的能力，但是Rust不支持创建自定义运算符，且只有定义在 `std::ops`中的运算符才能进行重载，上述例子没有实现 `Add<RHS>`特征，而是实现了 `Add`特征，这意味着我们使用了RHS的默认类型也就是Self,或者说，此处定义的是两个相同的Point类型相加，无需指定RHS.

如下例子与上述例子相反，创建了两个不同类型的相加：

```rust
use std::ops::Add;

struct Millimeters(u32);
struct Meters(u32);

impl Add<Meters> for Millimeters {
    type Output = Millimeters;

    fn add(self, other: Meters) -> Millimeters {
        Millimeters(self.0 + (other.0 * 1000))
    }
}
```

默认类型参数主要用于两个方面：

+ 减少实现的样板代码
+ 扩展类型但是无需大幅修改现有的代码

##### 8.4.3 调用同名方法

当类型与特征具有同名的方法的时候，编译器会优先调用类型中的方法

###### 8.4.3.1 **调用特征上的同名方法**

使用显式调用的语法，如下

```rust
fn main() {
    let person = Human;
    Pilot::fly(&person); // 调用Pilot特征上的方法
    Wizard::fly(&person); // 调用Wizard特征上的方法
    person.fly(); // 调用Human类型自身的方法
}
// 注意特征上的fly()参数为&self
```

如果是关联函数，也即没有 `&self`参数的时候呢？使用**完全限定语法**

###### 8.4.3.2 **完全限定语法**

```rust
<SpecificType as TraitName>::func_name(receiver_if_method, args...)
```

使用如上形式调用函数

在尖括号内，通过as关键字，向Rust编译器提供了类型注解，最终会调用

`impl TraitName for SpecificType`中的方法。

其中第一个参数是方法接收器 `receiver`（三种 `self`-> 1. `self` 值接收器 2.`&self`可借用的引用接收器 3. `&mut self` 可变引用接收器）,只有方法才拥有。

> 完全限定语法可以用于任何函数或方法调用，那么我们为何很少用到这个语法？原因是 Rust  编译器能根据上下文自动推导出调用的路径，因此大多数时候，我们都无需使用完全限定语法。只有当存在多个同名函数或方法，且 Rust  无法区分出你想调用的目标函数时，该用法才能真正有用武之地。

##### 8.4.4 特征定义中的特征约束

> 当需要让特征A使用特征B的功能时，不仅要为类型实现特征A,还需要为类型实现特征B
>
> 这里就要引入一个新概念 **基特征（super trait）**, 此处的特征B就被成为**基特征**

```rust
trait A: B {
    /// ...
}
```

如上，若要实现A特征，则需要先实现B特征

```rust
impl B for SpecificType {
    /// ...
}

impl A for SpecificType {
    /// ...
}
```

##### 8.4.5 在外部类型上实现外部特征（newtype）

> newtype 模式用于绕过**孤儿原则**
>
> 简而言之：就是为一个**元组结构体**创建新类型。该元组结构体封装有一个字段，该字段就是希望实现特征的具体类型。

```rust
use std::fmt;

struct Wrapper(Vec<String>);

impl fmt::Display for Wrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]", self.0.join(", "))
    }
}

fn main() {
    let w = Wrapper(vec![String::from("hello"), String::from("world")]);
    println!("w = {}", w);
}
```

这样一来，本来 `Vec(T)`和 `Display`特征都不在标准库中，无法直接为其实现Display,但是可以通过 `newtype`模式为其实现 `Display`，即上述方法。通过先创建一个类型，这样就摆脱了孤儿规则——即在本地有特征和类型其中一种。

---

### 9.集合类型

#### 9.1 动态数组 Vector

##### 9.1.1 创建动态数组

###### Vec::new

```rust
let v: Vec<i32> = Vec::new();
// or this way
let mut v: Vec::new();
v.push(1);
```

We could use `Vec::with_capacity(capacity)` if we know the capacity of the number of items, this could help performance promotion.

###### vec![] (macro)

```rust
let v = vec![1, 2, 3];
```

##### 9.1.2 Update Vector

Using **`push()`** Method (Should Declare as Mutable -> let mut xxx;)

> 与结构体一样，Vector 类型在超出作用域范围后，会被自动删除
>
> 其内部所有的内容也会随之被删除

##### 9.1.3 Read Elements from Vector

+ Index
+ get Method

```rust
let v = vec![1, 2, 3];
let third: &32 = &v[2];
println!("第三个元素是 {}", third);

v.get(2); // 返回一个 Option
```

使用Index可能会造成越界

使用 `.get()`方法返回一个Option（有值的时候返回 `Some(T)`，无值的时候返回 `None`）

##### 9.1.4 同时借用多个数组元素（切片？）

```rust
let mut v = vec![1, 2, 3, 4, 5];

let first = &v[0];

v.push(6);

println!("The first element is: {first}");

```

如上会发生错误，因为 `let first = &v[0];` 发生了不可变借用，而push方法又发生了可变借用，而first在push方法之后还会用到，所以可以通过交换这两行代码来避免报错

##### 9.1.5 迭代遍历 Vector 中的元素

```rust
let mut v = vec![1, 2, 3];
// 不可变借用
for i in &v{
    println!("{i}")
}
// 可变借用
for i in &mut v{
    // *i 代表解引用
    *i += 10;
}

for i in &v{
    println!("{i}")
}

```

##### 9.1.6 存储不同元素

可以使用枚举套娃来实现（即在不同类型上套一层同样的枚举类型）

也可以用特征对象来实现

```rust
trait IpAddr {
    fn display(&self);
}

struct V4(String);
impl IpAddr for V4 {
    fn display(&self) {
        println!("ipv4: {:?}",self.0)
    }
}
struct V6(String);
impl IpAddr for V6 {
    fn display(&self) {
        println!("ipv6: {:?}",self.0)
    }
}

fn main() {
    let v: Vec<Box<dyn IpAddr>> = vec![
        Box::new(V4("127.0.0.1".to_string())),
        Box::new(V6("::1".to_string())),
    ];

    for ip in v {
        ip.display();
    }
}
```

##### 9.1.7 Vector 常用方法

###### 初始化

```rust
fn main() {
	let v = vec![1;3];	// 3个1
    let v_from = Vec::from([0, 0, 0]); // 从已有数组初始化
}
```

> 除此之外
>
> `vec!(..)` 和 `vec![..]` 是同样的宏，宏可以使用 []、()、{}三种形式

例子1：

```rust
fn main() {
    let mut v = Vec::with_capacity(10);
    v.extend([1, 2, 3]);    // 附加数据到 v
    println!("Vector 长度是: {}, 容量是: {}", v.len(), v.capacity());

    v.reserve(100);        // 调整 v 的容量，至少要有 100 的容量
    println!("Vector（reserve） 长度是: {}, 容量是: {}", v.len(), v.capacity());

    v.shrink_to_fit();     // 释放剩余的容量，一般情况下，不会主动去释放容量
    println!("Vector（shrink_to_fit） 长度是: {}, 容量是: {}", v.len(), v.capacity());
}
```

例子2：

```rust
let mut v =  vec![1, 2];
assert!(!v.is_empty());         // 检查 v 是否为空

v.insert(2, 3);                 // 在指定索引插入数据，索引值不能大于 v 的长度， v: [1, 2, 3] 
assert_eq!(v.remove(1), 2);     // 移除指定位置的元素并返回, v: [1, 3]
assert_eq!(v.pop(), Some(3));   // 删除并返回 v 尾部的元素，v: [1]
assert_eq!(v.pop(), Some(1));   // v: []
assert_eq!(v.pop(), None);      // 记得 pop 方法返回的是 Option 枚举值
v.clear();                      // 清空 v, v: []

let mut v1 = [11, 22].to_vec(); // append 操作会导致 v1 清空数据，增加可变声明
v.append(&mut v1);              // 将 v1 中的所有元素附加到 v 中, v1: []
v.truncate(1);                  // 截断到指定长度，多余的元素被删除, v: [11]
v.retain(|x| *x > 10);          // 保留满足条件的元素，即删除不满足条件的元素

let mut v = vec![11, 22, 33, 44, 55];
// 删除指定范围的元素，同时获取被删除元素的迭代器, v: [11, 55], m: [22, 33, 44]
let mut m: Vec<_> = v.drain(1..=3).collect();  

let v2 = m.split_off(1);        // 指定索引处切分成两个 vec, m: [22], v2: [33, 44]
```

例子3：（数组切片的方式）

```rust
fn main() {
    let v = vec![11, 22, 33, 44, 55];
    let slice = &v[1..=3];
    assert_eq!(slice, &[22, 33, 44]);
}
```

##### 9.1.8 Vector 的排序

+ 稳定排序：
  + sort
  + sort_by
+ 非稳定排序：
  + sort_unstable
  + sort_unstable_by

###### 整数数组的排序

```rust
fn main() {
    let mut vec = vec![1, 5, 10, 2, 15];  
    vec.sort_unstable();  
    assert_eq!(vec, vec![1, 2, 5, 10, 15]);
}
// 正常执行无报错
```

###### 浮点数数组的排序

```rust
fn main() {
    let mut vec = vec![1.0, 5.6, 10.3, 2.0, 15f32];  
    vec.sort_unstable();  
    assert_eq!(vec, vec![1.0, 2.0, 5.6, 10.3, 15f32]);
}
// 会报错，因为浮点数中存在一个NAN值，无法与其他的浮点数进行比较所以没有实现Trait Ord,
// 只实现了部分可比较的特性 PartialOrd
```

所以使用 `partial_cmp`来排序

###### ⚠todo！

```rust
fn main() {
    let mut vec = vec![1.0, 5.6, 10.3, 2.0, 15f32];  
    vec.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());  
    assert_eq!(vec, vec![1.0, 2.0, 5.6, 10.3, 15f32]);
}
```

###### 结构体数组的排序

```rust
#[derive(Debug)]
struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: String, age: u32) -> Person {
        Person { name, age }
    }
}

fn main() {
    let mut people = vec![
        Person::new("Zoe".to_string(), 25),
        Person::new("Al".to_string(), 60),
        Person::new("John".to_string(), 1),
    ];
    // 定义一个按照年龄倒序排序的对比函数
    people.sort_unstable_by(|a, b| b.age.cmp(&a.age));

    println!("{:?}", people);
}
```

#### 9.2 KV 存储 HashMap

##### 9.2.1 创建HashMap

```rust
// new方法
use std::collections::HashMap;

let mut my_gems = HashMap::new();

my_gems.insert("红宝石", 1);
my_gems.insert("蓝宝石", 2);
my_gems.insert("河边捡的误以为是宝石的破石头", 18);
// 和 Vec一样如果预先知道要存储的KV对个数，可以使用HashMap::with_capacity(capacity)创建指定大小的HashMap，以此来提升性能

fn main() {
    use std::collections::HashMap;

    let teams_list = vec![
        ("中国队".to_string(), 100),
        ("美国队".to_string(), 10),
        ("日本队".to_string(), 50),
    ];

    let teams_map: HashMap<_,_> = teams_list.into_iter().collect();
    // into_iter() 方法将列表（数组）转换为迭代器，接着通过collect()进行收集
    // 同时需要标注类型 HashMap<_,_>
    println!("{:?}",teams_map)
}
```

##### 9.2.2 所有权转移

+ 若类型实现Copy特征，则该类型会被复制进HashMap
+ 若没有实现Copy特征，所有权将被转移给HashMap中

**如果使用引用类型放入 HashMap 中**，请确保该引用的生命周期至少跟 `HashMap` 活得一样久（否则会出错）

##### 9.2.3 查询 HashMap

```rust
// 通过 get 方法获取元素
use std::collections::HashMap;

let mut scores = HashMap::new();

scores.insert(String::from("Blue"), 10);
scores.insert(String::from("Yellow"), 50);

let team_name = String::from("Blue");
let score: Option<&i32> = scores.get(&team_name);
```

+ `get`方法返回一个 `Option<&i32>`类型：查询不到就会返回 `None`,查询到了返回 `Some(&i32)`
+ `&i32`是对HashMap中值的借用，如果不使用借用，可能会发生所有权的转移

###### 通过循环的方式依次遍历KV对

```rust
use std::collections::HashMap;

let mut scores = HashMap::new();

scores.insert(String::from("Blue"), 10);
scores.insert(String::from("Yellow"), 50);

for (key, value) in &scores {
    println!("{}: {}", key, value);
}

```

##### 9.2.4 **Option中的copied方法和unwrap_or方法**

###### ⚠TODO!

+ `copied`方法

  + 含义：
  + 用法：
  + 例子：
+ `unwrap_or`方法

  + 含义：
  + 用法：
  + 例子：

##### 9.2.5 更新HashMap中的值

```rust
fn main() {
    use std::collections::HashMap;

    let mut scores = HashMap::new();

    scores.insert("Blue", 10);

    // 覆盖已有的值
    let old = scores.insert("Blue", 20);	// 此处old会获取上一次的值
    assert_eq!(old, Some(10));

    // 查询新插入的值
    let new = scores.get("Blue");			// 此处new会获取到更新后的值
    assert_eq!(new, Some(&20));

    // 查询Yellow对应的值，若不存在则插入新值
    let v = scores.entry("Yellow").or_insert(5);	// 不存在，所以此处v会获取到插入的值
    assert_eq!(*v, 5); // 不存在，插入5

    // 查询Yellow对应的值，若不存在则插入新值
    let v = scores.entry("Yellow").or_insert(50);	// 此处v会获取到存在的值
    assert_eq!(*v, 5); // 已经存在，因此50没有插入
}
```

###### 在已有值的基础上更新

```rust
use std::collections::HashMap;

let text = "hello world wonderful world";

let mut map = HashMap::new();
// 根据空格来切分字符串(英文单词都是通过空格切分)
for word in text.split_whitespace() {
    let count = map.entry(word).or_insert(0);
    *count += 1;
}

println!("{:?}", map);
```

**注意：**

+ `or_insert`返回了 `&mut v`引用，因此可以通过该可变引用来直接修改 `map`中对应的值
+ 使用 `count`引用时，需要先进行解引用 `*count`

##### 9.2.6 哈希函数

一个类型需要实现 `std::cmp::Eq`特征，才可以作为 `Key`因为需要两个Key可以互相比较看是否相等。

不同的Key不能映射到相同的值！（这种情况称为哈希碰撞或者散列碰撞，需要用其他的方法来解决，开放寻址法或者链地址法）

像f32和f64没有实现std::cmp::Eq特征，所以不能用作HashMap的Key

> 若要追求安全，尽可能减少冲突，同时防止拒绝服务（Denial of Service, DoS）攻击，就要使用密码学安全的哈希函数，`HashMap` 就是使用了这样的哈希函数。反之若要追求性能，就需要使用没有那么安全的算法。

###### 高性能第三方库

> 目前，`HashMap` 使用的哈希函数是 `SipHash`，它的性能不是很高，但是安全性很高。`SipHash` 在中等大小的 `Key` 上，性能相当不错，但是对于小型的 `Key` （例如整数）或者大型 `Key` （例如字符串）来说，性能还是不够好。若你需要极致性能，例如实现算法，可以考虑这个库：[ahash](https://github.com/tkaitchuck/ahash)。

---

### 10.认识生命周期

#### ⚠TODO！

---

### 11.返回值和错误处理

#### 11.1 `panic` 深入剖析

##### 11.1.1 `panic!` 与不可恢复错误

###### 被动触发

如数组越界

###### 主动调用

即在代码中主动调用 `panic!`

##### 11.1.2 `backtrace`栈展开

在使用时加上一个环境变量可以获取更详细的栈展开信息：

- Linux/macOS 等 UNIX 系统： `RUST_BACKTRACE=1 cargo run`
- Windows 系统（PowerShell）： `$env:RUST_BACKTRACE=1 ; cargo run`

得到的代码就是依次栈展开（也称栈回溯），包含了函数调用的顺序（按照逆序排列）：最近调用的函数排在列表最上方

排在最顶部的最后一个调用的函数是 `rust_begin_unwind`

要获取到栈回溯信息，你还需要开启 `debug` 标志，该标志在使用 `cargo run` 或者 `cargo build` 时自动开启（这两个操作默认是 `Debug` 运行方式）。同时，栈展开信息在不同操作系统或者 Rust 版本上也有所不同。

##### 11.1.3 `panic` 时的两种终止方式

+ 栈展开（默认方式）

  回溯栈上数据和函数调用，因此意味着更多的善后工作，好处是可以给出充分的报错信息和栈调用信息，便于事后的问题复盘
+ 直接终止

  不清理数据直接退出程序，善后工作交与操作系统来负责

  当关心最终编译出的二进制可执行文件大小时，可以尝试去使用直接终止的方式，例如下面的配置修改 `Cargo.toml` 文件，实现在 `release`手动编译和运行项目 模式下遇到 `panic` 直接终止：

  ```toml
  [profile.release]
  panic = 'abort'
  ```

##### 11.1.4 线程 `panic` 后，程序是否会终止？

如果是main线程，则程序终止，如果是其它子线程，该线程终止，但是不会影响main线程，因此不要尽量在main线程中做太多任务，交由子线程去做，这样就算子线程panic也不会导致整个程序结束

##### 11.1.5 何时该使用 `panic!`

首先展示一下Result枚举类型：

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

当没有错误发生时，函数返回一个用Reult类型包裹的值Ok(T)，当错误时，返回一个Err(E)，对于Resule返回有很多处理方法，最简单粗暴的就是 `unwrap`和 `expect`

以 `unwrap` 举例：

```rust
use std::net::IpAddr;
let home: IpAddr = "127.0.0.1".parse().unwrap();
```

上面的 `parse` 方法试图将字符串 `"127.0.0.1" `解析为一个 IP 地址类型 `IpAddr`，它返回一个 `Result<IpAddr, E>` 类型，如果解析成功，则把 `Ok(IpAddr)` 中的值赋给 `home`，如果失败，则不处理 `Err(E)`，而是直接 `panic`。

因此 `unwrap` 简而言之：成功则返回值，失败则 `panic`，总之不进行任何错误处理。

当代码确定是正确时，可以用 `unwrap`方法直接处理，因为不可能 `panic`

如果该字符串是来自于用户输入，那在实际项目中，就必须用错误处理的方式，而不是 `unwrap`，否则程序崩溃次数无法计量。

###### 可能导致全局有害状态时

有害状态大概分为几类：

- 非预期的错误
- 后续代码的运行会受到显著影响
- 内存安全的问题

当错误预期会出现时，返回一个错误较为合适

##### 11.1.6 `panic` 原理分析

当调用 `panic!` 宏时，它会

1. 格式化 `panic` 信息，然后使用该信息作为参数，调用 `std::panic::panic_any()` 函数
2. `panic_any` 会检查应用是否使用了 `panic hook` ，如果使用了，该 `hook` 函数就会被调用（`hook` 是一个钩子函数，是外部代码设置的，用于在 `panic` 触发时，执行外部代码所需的功能）
3. 当 `hook` 函数返回后，当前的线程就开始进行栈展开：从 `panic_any` 开始，如果寄存器或者栈因为某些原因信息错乱了，那很可能该展开会发生异常，最终线程会直接停止，展开也无法继续进行
4. 展开的过程是一帧一帧的去回溯整个栈，每个帧的数据都会随之被丢弃，但是在展开过程中，你可能会遇到被用户标记为 `catching` 的帧（通过 `std::panic::catch_unwind()` 函数标记），此时用户提供的 `catch` 函数会被调用，展开也随之停止：当然，如果 `catch` 选择在内部调用 `std::panic::resume_unwind()` 函数，则展开还会继续。

还有一种情况，在展开过程中，如果展开本身 `panic` 了，那展开线程会终止，展开也随之停止。

一旦线程展开被终止或者完成，最终的输出结果是取决于哪个线程 `panic`：对于 `main` 线程，操作系统提供的终止功能 `core::intrinsics::abort()` 会被调用，最终结束当前的 `panic` 进程；如果是其它子线程，那么子线程就会简单的终止，同时信息会在稍后通过 `std::thread::join()` 进行收集。

#### 11.2 返回值 `Result` 和 `?` (可恢复的错误)

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

泛型参数 `T` 代表成功时存入的正确值的类型，存放方式是 `Ok(T)`，`E` 代表错误时存入的错误值，存放方式是 `Err(E)`

例子

```rust
use std::fs::File;

fn main() {
    let f = File::open("hello.txt");
}
```

以上 `File::open` 返回一个 `Result` 类型

```rust
use std::fs::File;

fn main() {
    let f = File::open("hello.txt");

    let f = match f {
        Ok(file) => file,
        Err(error) => {
            panic!("Problem opening the file: {:?}", error)
        },
    };
}
```

##### 11.2.1 对返回的错误进行处理

```rust
use std::fs::File;
use std::io::ErrorKind;

fn main() {
    let f = File::open("hello.txt");

    let f = match f {
        Ok(file) => file,
        Err(error) => match error.kind() {
            ErrorKind::NotFound/* 文件不存在错误，则创建文件 */ => match File::create("hello.txt") {
                Ok(fc) => fc,
                Err(e) => panic!("Problem creating the file: {:?}", e),
            },
            other_error => panic!("Problem opening the file: {:?}", other_error),
        },
    };
}
```

- 如果是文件不存在错误 `ErrorKind::NotFound`，就创建文件，这里创建文件 `File::create` 也是返回 `Result`，因此继续用 `match` 对其结果进行处理：创建成功，将新的文件句柄赋值给 `f`，如果失败，则 `panic`
- 剩下的错误，一律 `panic`

##### 11.2.2 失败就 `panic: unwrap` 和 `expect`

`unwrap()`直接将result中的值取出，如果是 `Ok`就取出，如果是 `Err`就直接 `panic`，直接崩溃

`expect` 跟 `unwrap` 很像，也是遇到错误直接 `panic`, 但是会带上自定义的错误提示信息，相当于重载了错误打印的函数：

```rust
use std::fs::File;

fn main() {
    let f = File::open("hello.txt").expect("Failed to open hello.txt");
}
```

`expect` 相比 `unwrap` 能提供更精确的错误信息，在有些场景也会更加实用。

##### 11.2.3 传播错误

指被调用的函数将错误一层一层网上传给调用链的上游函数进行处理

###### `?`

Example

```rust
use std::fs::File;
use std::io;
use std::io::Read;

fn read_username_from_file() -> Result<String, io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}
```

`?` 作用就相当于

```rust
let mut f = match f {
    // 打开文件成功，将file句柄赋值给f
    Ok(file) => file,
    // 打开文件失败，将错误返回(向上传播)
    Err(e) => return Err(e),
};
```

如果结果是 `Ok(T)`，则把 `T` 赋值给 `f`，如果结果是 `Err(E)`，则返回该错误，所以 `?` 特别适合用来传播错误。

`?` 也可以自动进行类型提升（转换）

```rust
fn open_file() -> Result<File, Box<dyn std::error::Error>> {
    let mut f = File::open("hello.txt")?;
    Ok(f)
}
```

上面代码中 `File::open` 报错时返回的错误是 `std::io::Error` 类型，但是 `open_file` 函数返回的错误类型是 `std::error::Error` 的特征对象，可以看到一个错误类型通过 `?` 返回后，变成了另一个错误类型，这就是 `?` 的神奇之处。

根本原因是 `?` 会自动调用 `From` 特征中的 `from` 方法，然后进行隐式类型转换，因此只要函数返回的错误 `ReturnError` 实现了 `From<OtherError>` 特征， `?` 就会自动把 `OtherError` 转换为 `ReturnError`

与用于返回 `Result` 的 `Err` 和 `Ok` 类似

###### `?` 也可用于返回 `Option` 的 `Some` 和 `None`

```rust
fn last_char_of_first_line(text: &str) -> Option<char> {
    text.lines().next()?.chars().last()
}
```

上面代码展示了在链式调用中使用 `?` 提前返回 `None` 的用法， `.next` 方法返回的是 `Option` 类型：如果返回 `Some(&str)`，那么继续调用 `chars` 方法，如果返回 `None`，则直接从整个函数中返回 `None`，不再继续进行链式调用。

```rust
fn first(arr: &[i32]) -> Option<&i32> {
   arr.get(0)?
}
```

这段代码无法通过编译，切记：? 操作符需要一个变量来承载正确的值，这个函数只会返回 Some(&i32) 或者 None，只有错误值能直接返回，正确的值不行，所以如果数组中存在 0 号元素，那么函数第二行使用 ? 后的返回类型为 &i32 而不是 Some(&i32)。因此 ? 只能用于以下形式：

```rust
let v = xxx()?;
xxx()?.yyy()?;
```

###### `try!`

```rust
macro_rules! try {
    ($e:expr) => (match $e {
        Ok(val) => val,
        Err(err) => return Err(::std::convert::From::from(err)),
    });
}
```

用法（对比）：

```rust
//  `?`
let x = function_with_error()?; // 若返回 Err, 则立刻返回；若返回 Ok(255)，则将 x 的值设置为 255

// `try!()`
let x = try!(function_with_error());
```

但是 `try!` 要避免使用，因为 `?` 不仅代码量更少，还可以做链式调用

`map` & `and_then`

```rust
pub fn map<U, F>(self, op: F) -> Result<U, E>
where
    F: FnOnce(T) -> U,
```

Maps a Result<T, E> to Result<U, E> by applying a function to a contained Ok value, leaving an Err value untouched.

例子

```rust
use std::num::ParseIntError;

// 使用两种方式填空: map, and then
fn add_two(n_str: &str) -> Result<i32, ParseIntError> {
   n_str.parse::<i32>().map(|x| x+2)
}

fn main() {
    assert_eq!(add_two("4").unwrap(), 6);

    println!("Success!")
}
```

```rust
pub fn and_then<U, F>(self, op: F) -> Result<U, E>
where
    F: FnOnce(T) -> Result<U, E>,
```

Calls op if the result is Ok, otherwise returns the Err value of self.

This function can be used for control flow based on Result values.

例子

```rust
use std::num::ParseIntError;

// 使用两种方式填空: map, and then
fn add_two(n_str: &str) -> Result<i32, ParseIntError> {
   n_str.parse::<i32>().and_then(|x| Ok(x+2))
}

fn main() {
    assert_eq!(add_two("4").unwrap(), 6);

    println!("Success!")
}
```

---

### 12.包和模块 Crates & Module

> 三个基本概念
>
> 项目：Packages 一个Cargo提供的 feature(特征)，可以用于构建、测试和分享包
>
> 包：Crate 一个由多个模块组成的树形结构，可以作为三方库进行分发，也可以生成可执行文件进行运行
>
> 模块：Module 可以一个文件多个模块，也可以一个文件一个模块，模块可以被认为是真实项目中的代码组织单元

#### 12.1 包和 Package

> 不同包之间可以由同名的类型，对于编译器而言，两者的边界非常清晰，不会存在引用歧义

Crate 被编译后会生成一个可执行文件或者一个库。对于Rust来说，包就是一个独立的可编译单元

> 由于Rust起名比较标新立异，包的名称被crate所占用，库的名称被library占用，所以Rust圣经将Package翻译成项目，也可以理解为工程、软件包

因为是一个项目，所以Package由独立的Cargo.toml，以及一些被组织在一起的包（一个或多个）。一个Package智能包含一个库（library）类型的包，但是可以包含多个二进制可执行类型的包。

```bash
# 创建一个库类型的 package
cargo new my-lib --lib
# 创建一个二进制类型（应用类型）的 package
cargo new my-project
```

> 如果一个Package 包含有 src/lib.rs 意味着它包含有一个库类型的同名包，该包的根文件是src/lib.rs
>
> 同上，如果一个Package 包含有 src/main.rs, 则意味着它有一个二进制类型的同名包

如果一个Package同时含有 src/main.rs 和 src/lib.rs ，就意味着它包含两个包：库包和二进制包两个包的包名与Package同名

一般标准Rust项目录结构如下：

+ 唯一库包：src/lib.rs
+ 默认二进制包：src/main.rs
+ 其余二进制包：src/bin/*.rs
+ 集成测试文件：tests目录下
+ 基准性能测试：benchmark文件：benches目录下
+ 项目示例：examples目录下

#### 12.2 模块 Module

##### 12.2.1 创建嵌套模块

形如下方代码即为嵌套模块：

```rust
// src/lib.rs
// 餐厅前厅，用于吃饭
mod front_of_house {
    mod hosting {
        fn add_to_waitlist() {}

        fn seat_at_table() {}
    }

    mod serving {
        fn take_order() {}

        fn serve_order() {}

        fn take_payment() {}
    }
}

pub fn eat_at_restaurant() {
    // 绝对路径
    crate::front_of_house::hosting::add_to_waitlist();

    // 相对路径
    front_of_house::hosting::add_to_waitlist();
}
```

```bash
# 上述嵌套模块的结构如下所示
crate(lib.rs)
 └── front_of_house
     ├── hosting
     │   ├── add_to_waitlist
     │   └── seat_at_table
     └── serving
         ├── take_order
         ├── serve_order
         └── take_payment

```

##### 12.2.2 用路径引用模块

+ **绝对路径**，从包根开始，路径名以包名或者 `crate` 作为开头
+ **相对路径**，从当前模块开始，以 `self`，`super` 或当前模块的标识符作为开头

上面代码中的 `eat_at_restaurant()`内部就有[**示例**](#####12.2.1 创建嵌套模块)

##### 12.2.3 代码可见性

**pub 关键字**

在访问某个模块内部的模块时，因为默认是private,即被隐藏了，要访问需要在该模块前面添加pub关键字（如果内部有函数要访问，内部的函数也一样要添加pub）

##### 12.2.4 使用 `super` 引用模块

在相对路径引用模块的方式中，可以用super来指代父模块，有点类似文件系统中的 `..`路径

##### 12.2.5 使用 `self` 引用模块

即引用自身模块中的项（自身所在文件中）

##### 12.2.6 结构体和枚举的可见性

- 将结构体设置为 `pub`，但它的所有字段依然是私有的
- 将枚举设置为 `pub`，它的所有字段也将对外可见

> 枚举和结构体的使用方式不一样。如果枚举的成员对外不可见，那该枚举将一点用都没有，因此枚举成员的可见性自动跟枚举可见性保持一致，这样可以简化用户的使用。
>
> 而结构体的应用场景比较复杂，其中的字段也往往部分在 A 处被使用，部分在 B 处被使用，因此无法确定成员的可见性，那索性就设置为全部不可见，将选择权交给程序员。

##### 12.2.7 模块与文件分离

当模块变多或者变大时，需要将模块放入一个单独的文件中，让代码更好维护。

如果需要将文件夹作为一个模块，我们需要进行显示指定暴露哪些子模块。按照上述的报错信息，我们有两种方法：

- 在 `divsion_mod` 目录里创建一个 `mod.rs`，如果你使用的 `rustc` 版本 `1.30` 之前，这是唯一的方法。
- 在 `division_mod` **同级**目录里创建一个与模块（目录）**同名**的 rs 文件 `division_mod.rs`，在新版本里，更建议使用这样的命名方式来避免项目中存在大量同名的 `mod.rs` 文件（ Python 点了个 `踩`）。

如果使用第二种方式，文件结构将如下所示：

```
src
├── division_mod
│   └── xxx.rs
├── division_mod.rs
└── lib.rs
```

在上述完成之后，在另一个需要用到的文件里面

添加

```rust
mod division_mod
pub use crate::division_mod::xxx;
```

- `mod division_mod;` 告诉 Rust 从另一个和模块 `division_mod` 同名的文件中加载该模块的内容
- 使用绝对路径的方式来引用 `xxx` 模块：`crate::division_mod::xxx;`

#### 12.3 use

> 使用use 可以使代码简化，不需要再用类似
>
> crate::division_mod::xxx这样那么长的调用方式

##### 12.3.1 基本引入方式

**绝对路径和相对路径**

> 基本与模块相同

###### 引入模块还是函数

从使用简洁性来说，引入函数自然是更甚一筹，但是在某些时候，引入模块会更好：

- 需要引入同一个模块的多个函数
- 作用域中存在同名函数

Rust Course 建议：**优先使用最细粒度（引入函数、结构体等）的引用方式，如果引起了某种麻烦（例如前面两种情况），再使用引入模块的方式**。

##### 12.3.2 避免同名引用

###### 模块::函数

```rust
use std::fmt
use std::io
```

###### as 别名引用

```rust
use std::fmt
use std::io::Result as IoResult;
// 则Result 代表 std::fmt::Result
// IoResult 代表 std::io::Result
```

##### 12.3.3 引入项再导出

当外部模块项A被引入到当前模块中时，其可见性自动被设置为私有，如果希望允许其他外部代码引用模块项A,可以再次进行导出

```rust
mod front_of_house {
    pub mod hosting {
        pub fn add_to_waitlist() {}
    }
}

pub use crate::front_of_house::hosting; // 这里的pub就可以让外部模块使用该模块时，也可以使用

pub fn eat_at_restaurant() {
    hosting::add_to_waitlist();
    hosting::add_to_waitlist();
    hosting::add_to_waitlist();
}
```

如上，使用 `pub use`即可实现目的。

这里 `use`代表引入 `hosting`模块到当前作用域，`pub`表示将该引入的内容再度设置为可见。

##### 12.3.4 使用第三方包

1. 现在 `Cargo.toml`文件内的 `[dependencies]`区域添加一行：`xxx_mod = "x.x.x"`
2. 此时如果IDE有合适的配置，就会自动拉取该库。

然后在代码中：

```rust
use xxx_mod::xxx_func_set;

fn main() {
	let xxx_val = xxx_func_inside::xxx_func();
}
// xxx_func_inside 是xxx_func_set内部的模块
// xxx_func 是xxx_func_inside内部的函数
```

###### `crates.io`，`lib.rs`

Rust 社区已经为我们贡献了大量高质量的第三方包，你可以在 `crates.io` 或者 `lib.rs` 中检索和使用，从目前来说查找包更推荐 `lib.rs`，搜索功能更强大，内容展示也更加合理，但是下载依赖包还是得用 `crates.io`。

可以在网站上搜索 `rand` 包，看看它的文档使用方式是否和我们之前引入方式相一致：在网上找到想要的包，然后将你想要的包和版本信息写入到 `Cargo.toml` 中。

##### 12.3.5 使用 {} 简化引入方式

```rust
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::collections::HashSet;

use std::cmp::Ordering;
use std::io;
```

优化为

```rust
use std::collections::{HashMap,BTreeMap,HashSet};
use std::{cmp::Ordering, io};
```

然后同时引入模块和模块中的项时：

```rust
use std::io;
use std::io::Write;
```

可以优化为

```rust
use std::io::{self, Write};
```

###### self

上面使用到了模块章节提到的 `self` 关键字，用来替代模块自身，结合上一节中的 `self`，可以得出它在模块中的两个用途：

- `use self::xxx`，表示加载当前模块中的 `xxx`。此时 `self` 可省略
- `use xxx::{self, yyy}`，表示，加载当前路径下模块 `xxx` 本身，以及模块 `xxx` 下的 `yyy`

##### 12.3.6 使用*引入模块下的所有项

```rust
use std::collections::*
```

使用*要小心，因为不知道会不会引用到同名的模块或函数导致冲突，对于编译器来说，本体同名类型的优先级更高

##### 12.3.7 受限的可见性

如果我们想要让某一项可以在整个包中都可以被使用，那么有两种办法：

- 在包根中定义一个非 `pub` 类型的 `X`(父模块的项对子模块都是可见的，因此包根中的项对模块树上的所有模块都可见)
- 在子模块中定义一个 `pub` 类型的 `Y`，同时通过 `use` 将其引入到包根

```rust
mod a {
    pub mod b {
        pub fn c() {
            println!("{:?}",crate::X);
        }

        #[derive(Debug)]
        pub struct Y;
    }
}

#[derive(Debug)]
struct X;
use a::b::Y;
fn d() {
    println!("{:?}",Y);
}
```

有时我们会遇到这两种方法都不太好用的时候。例如希望对于某些特定的模块可见，但是对于其他模块又不可见：

```rust
// 目标：`a` 导出 `I`、`bar` and `foo`，其他的不导出
pub mod a {
    pub const I: i32 = 3;

    fn semisecret(x: i32) -> i32 {
        use self::b::c::J;
        x + J
    }

    pub fn bar(z: i32) -> i32 {
        semisecret(I) * z
    }
    pub fn foo(y: i32) -> i32 {
        semisecret(I) + y
    }

    mod b {
        mod c {
            const J: i32 = 4;
        }
    }
}
```

子模块看得到父模块的项，父模块看不到子模块的项，所以这段代码会报错，semisecret方法中，`a` -> `b` -> `c` 形成了父子模块链，那 `c` 中的 `J` 自然对 `a` 模块不可见。

如果想保持代码逻辑，同时又只让 `J` 在 `a` 内可见该怎么办？

```rust
pub mod a {
    pub const I: i32 = 3;

    fn semisecret(x: i32) -> i32 {
        use self::b::c::J;
        x + J
    }

    pub fn bar(z: i32) -> i32 {
        semisecret(I) * z
    }
    pub fn foo(y: i32) -> i32 {
        semisecret(I) + y
    }

    mod b {
        pub(in crate::a) mod c {
            pub(in crate::a) const J: i32 = 4;
        }
    }
}
```

通过 `pub(in crate::a)` 的方式，我们指定了模块 `c` 和常量 `J` 的可见范围都只是 `a` 模块中，`a` 之外的模块是完全访问不到它们的。

###### 限制可见性语法

`pub(crate)` 或 `pub(in crate::a)` 就是限制可见性语法，前者是限制在整个包内可见，后者是通过绝对路径，限制在包内的某个模块内可见，总结一下：

- `pub` 意味着可见性无任何限制
- `pub(crate)` 表示在当前包可见
- `pub(self)` 在当前模块可见
- `pub(super)` 在父模块可见
- `pub(in <path>)` 表示在某个路径代表的模块中可见，其中 `path` 必须是父模块或者祖先模块

###### 综合示例

```rust
// 一个名为 `my_mod` 的模块
mod my_mod {
    // 模块中的项默认具有私有的可见性
    fn private_function() {
        println!("called `my_mod::private_function()`");
    }

    // 使用 `pub` 修饰语来改变默认可见性。
    pub fn function() {
        println!("called `my_mod::function()`");
    }

    // 在同一模块中，项可以访问其它项，即使它是私有的。
    pub fn indirect_access() {
        print!("called `my_mod::indirect_access()`, that\n> ");
        private_function();
    }

    // 模块也可以嵌套
    pub mod nested {
        pub fn function() {
            println!("called `my_mod::nested::function()`");
        }

        #[allow(dead_code)]
        fn private_function() {
            println!("called `my_mod::nested::private_function()`");
        }

        // 使用 `pub(in path)` 语法定义的函数只在给定的路径中可见。
        // `path` 必须是父模块（parent module）或祖先模块（ancestor module）
        pub(in crate::my_mod) fn public_function_in_my_mod() {
            print!("called `my_mod::nested::public_function_in_my_mod()`, that\n > ");
            public_function_in_nested()
        }

        // 使用 `pub(self)` 语法定义的函数则只在当前模块中可见。
        pub(self) fn public_function_in_nested() {
            println!("called `my_mod::nested::public_function_in_nested");
        }

        // 使用 `pub(super)` 语法定义的函数只在父模块中可见。
        pub(super) fn public_function_in_super_mod() {
            println!("called my_mod::nested::public_function_in_super_mod");
        }
    }

    pub fn call_public_function_in_my_mod() {
        print!("called `my_mod::call_public_funcion_in_my_mod()`, that\n> ");
        nested::public_function_in_my_mod();
        print!("> ");
        nested::public_function_in_super_mod();
    }

    // `pub(crate)` 使得函数只在当前包中可见
    pub(crate) fn public_function_in_crate() {
        println!("called `my_mod::public_function_in_crate()");
    }

    // 嵌套模块的可见性遵循相同的规则
    mod private_nested {
        #[allow(dead_code)]
        pub fn function() {
            println!("called `my_mod::private_nested::function()`");
        }
    }
}

fn function() {
    println!("called `function()`");
}

fn main() {
    // 模块机制消除了相同名字的项之间的歧义。
    function();
    my_mod::function();

    // 公有项，包括嵌套模块内的，都可以在父模块外部访问。
    my_mod::indirect_access();
    my_mod::nested::function();
    my_mod::call_public_function_in_my_mod();

    // pub(crate) 项可以在同一个 crate 中的任何地方访问
    my_mod::public_function_in_crate();

    // pub(in path) 项只能在指定的模块中访问
    // 报错！函数 `public_function_in_my_mod` 是私有的
    //my_mod::nested::public_function_in_my_mod();
    // 试一试 ^ 取消该行的注释

    // 模块的私有项不能直接访问，即便它是嵌套在公有模块内部的

    // 报错！`private_function` 是私有的
    //my_mod::private_function();
    // 试一试 ^ 取消此行注释

    // 报错！`private_function` 是私有的
    //my_mod::nested::private_function();
    // 试一试 ^ 取消此行的注释

    // 报错！ `private_nested` 是私有的
    //my_mod::private_nested::function();
    // 试一试 ^ 取消此行的注释
}
```

---

### 13.注释和文档

#### 13.1 注释的种类

+ 代码注释：用于说明某一块代码的功能
+ 文档注释：支持Markdown
+ 包和模块注释：主要用于说明当前包和模块的功能

#### 13.2 代码注释

+ 行注释

  ```rust
  //
  ```
+ 块注释

  ```rust
  /* ... */
  ```

#### 13.3 文档注释

+ 文档行注释

  ```rust
  ///
  ```
+ 文档块注释

  ```rust
  /** ... */
  ```

##### 13.3.1 查看文档

```bash
cargo doc
```

执行如上命令会直接生成 `HTML`文件，放在 `target/doc`目录下

```bash
cargo doc --open
# 在生成文档后，自动在浏览器中打开网页
```

#### 13.4 包和模块级别的注释

+ 行注释

  ```rust
  //!
  ```
+ 块注释

  ```rust
  /*! ... */
  ```

**注意这些注释要放到包和模块的最上方！！！**

#### 13.5 文档测试（Doc Test）

Rust允许在文档注释中编写单元用例测试，如下：

```rust
/// `add_one` 将指定值加1
///
/// # Examples11
///
/// ```
/// let arg = 5;
/// let answer = world_hello::compute::add_one(arg);
///
/// assert_eq!(6, answer);
/// ```
pub fn add_one(x: i32) -> i32 {
    x + 1
}
```

#### 13.6 文档搜索别名

Rust 文档支持搜索功能，我们可以为自己的类型定义几个别名，以实现更好的搜索展现（在Rust doc网页中），当别名命中时，搜索结果会被放在第一位：

```rust
#[doc(alias = "x")]
#[doc(alias = "big")]
pub struct BigX;

#[doc(alias("y", "big"))]
pub struct BigY;
```

---

### 14.格式化输出

###### 简要示例

```rust
println!("Hello");                 // => "Hello"
println!("Hello, {}!", "world");   // => "Hello, world!"
println!("The number is {}", 1);   // => "The number is 1"
println!("{:?}", (3, 4));          // => "(3, 4)"
println!("{value}", value=4);      // => "4"
println!("{} {}", 1, 2);           // => "1 2"
println!("{:04}", 42);             // => "0042" with leading zeros
```

#### 14.1 `print!`, `println!`, `format!`

- `print!` 将格式化文本输出到标准输出，不带换行符
- `println!` 同上，但是在行的末尾添加换行符
- `format!` 将格式化文本输出到 `String` 字符串

##### 14.1.1 `eprint!`, `eprintln!`

> 它们仅应该被用于输出错误信息和进度信息，其它场景都应该使用 `print!` 系列。

#### 14.2 {} 与

与 `{}` 类似，`{:?}` 也是占位符：

- `{}` 适用于实现了 `std::fmt::Display` 特征的类型，用来以更优雅、更友好的方式格式化文本，例如展示给用户
- `{:?}` 适用于实现了 `std::fmt::Debug` 特征的类型，用于调试场景

与大部分类型实现了 `Debug` 不同，实现了 `Display` 特征的 Rust 类型并没有那么多，往往需要我们自定义想要的格式化方式

`{:#?}` 与 `{:?}` 几乎一样，唯一的区别在于它能更优美地输出内容

对于 `Display` 不支持的类型，可以考虑使用 `{:#?}` 进行格式化，虽然理论上它更适合进行调试输出。

##### 14.2.1 为自定义类型实现Display特征

如果类型是定义在当前作用域，可以为其实现 `Display`特征，可用于格式化输出：

```rust
struct Person {
    name: String,
    age: u8,
}

use std::fmt;
impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "大佬在上，请受我一拜，小弟姓名{}，年芳{}，家里无田又无车，生活苦哈哈",
            self.name, self.age
        )
    }
}
fn main() {
    let p = Person {
        name: "sunface".to_string(),
        age: 18,
    };
    println!("{}", p);
}
```

##### 14.2.2 为外部类型实现Display特征

在 Rust 中，无法直接为外部类型实现外部特征，但是可以使用 `newtype`解决此问题（孤儿规则）：

```rust
struct Array(Vec<i32>);

use std::fmt;
impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "数组是：{:?}", self.0)
    }
}
fn main() {
    let arr = Array(vec![1, 2, 3]);
    println!("{}", arr);
}
```

#### 14.3 位置参数

```rust
fn main() {
    println!("{}{}", 1, 2); // =>"12"
    println!("{1}{0}", 1, 2); // =>"21"
    // => Alice, this is Bob. Bob, this is Alice
    println!("{0}, this is {1}. {1}, this is {0}", "Alice", "Bob");
    println!("{1}{}{0}{}", 1, 2); // => 2112
}
```

#### 14.4 具名参数

```rust
fn main() {
    println!("{argument}", argument = "test"); // => "test"
    println!("{name} {}", 1, name = 2); // => "2 1"
    println!("{a} {c} {b}", a = "a", b = 'b', c = 3); // => "a 3 b"
}
```

需要注意的是：**带名称的参数必须放在不带名称参数的后面**，例如下面代码将报错：

```rust
println!("{abc} {1}", abc = "def", 2);
```

#### 14.5 格式化参数

> 对输出有更多的要求
>
> **可以观察到下面全部代码 `{}`占位符内都加上了 `:`**

```rust
fn main() {
    let v = 3.1415926;
    // Display => 3.14
    println!("{:.2}", v);
    // Debug => 3.14
    println!("{:.2?}", v);
}
```

##### 14.5.1 宽度

宽度用来指示输出目标的长度，如果长度不够，则进行填充和对齐：

###### 字符串填充

> 字符串格式化默认使用空格进行填充，并且进行左对齐，长度超过的话无视要求的对齐长度（按原格式继续输出）

```rust
fn main() {
    //-----------------------------------
    // 以下全部输出 "Hello x    !"
    // 为"x"后面填充空格，补齐宽度5
    println!("Hello {:5}!", "x");
    // 使用参数5来指定宽度
    println!("Hello {:1$}!", "x", 5);
    // 使用x作为占位符输出内容，同时使用5作为宽度
    println!("Hello {1:0$}!", 5, "x");
    // 使用有名称的参数作为宽度
    println!("Hello {:width$}!", "x", width = 5);
    //-----------------------------------

    // 使用参数5为参数x指定宽度，同时在结尾输出参数5 => Hello x    !5
    println!("Hello {:1$}!{}", "x", 5);
}
```

###### 数字填充：符号和0

> 数字格式化默认也是使用空格进行填充，但与字符串左对齐不同的是，数字是右对齐。

```rust
fn main() {
    // 宽度是5 => Hello     5!
    println!("Hello {:5}!", 5);
    // 显式的输出正号 => Hello +5!
    println!("Hello {:+}!", 5);
    // 宽度5，使用0进行填充 => Hello 00005!
    println!("Hello {:05}!", 5);
    // 负号也要占用一位宽度 => Hello -0005!
    println!("Hello {:05}!", -5);
}
```

##### 14.5.2 对齐

左对齐就是把原格式往左边顶，右边进来补齐的内容

右对齐就是把原格式往右边顶，左边进来补齐的内容

```rust
fn main() {
    // 以下全部都会补齐5个字符的长度
    // 左对齐 => Hello x    !
    println!("Hello {:<5}!", "x");
    // 右对齐 => Hello     x!
    println!("Hello {:>5}!", "x");
    // 居中对齐 => Hello   x  !
    println!("Hello {:^5}!", "x");

    // 对齐并使用指定符号填充 => Hello x&&&&!
    // 指定符号填充的前提条件是必须有对齐字符
    println!("Hello {:&<5}!", "x");
}
```

##### 14.5.3 精度

> 控制浮点数的精度或者字符串的长度

```rust
fn main() {
    let v = 3.1415926;
    let miv = -3.1415926;
    // 保留小数点后两位 => 3.14
    println!("{:.2}", v);
    // 带符号保留小数点后两位 => +3.14
    println!("{:+.2}", v);
    // => -3.14
    println!("{:+.2}", miv)
    // 不带小数 => 3
    println!("{:.0}", v);
    // 通过参数来设定精度 => 3.1416，相当于{:.4}
    println!("{:.1$}", v, 4);

    let s = "hi我是Sunface孙飞";
    // 保留字符串前三个字符 => hi我
    println!("{:.3}", s);
    // {:.*}接收两个参数，第一个是精度，第二个是被格式化的值 => Hello abc!
    println!("Hello {:.*}!", 3, "abcdefg");
}
```

##### 14.5.3 进制

可以使用 `#` 号来控制数字的进制输出：

- `#b`, 二进制
- `#o`, 八进制
- `#x`, 小写十六进制
- `#X`, 大写十六进制
- `x`, 不带前缀的小写十六进制

```rust
fn main() {
    // 二进制 => 0b11011!
    println!("{:#b}!", 27);
    // 八进制 => 0o33!
    println!("{:#o}!", 27);
    // 十进制 => 27!
    println!("{}!", 27);
    // 小写十六进制 => 0x1b!
    println!("{:#x}!", 27);
    // 大写十六进制 => 0x1B!
    println!("{:#X}!", 27);

    // 不带前缀的十六进制 => 1b!
    println!("{:x}!", 27);

    // 使用0填充二进制，宽度为10 => 0b00011011!
    println!("{:#010b}!", 27);
}
```

##### 14.5.4 指数

```rust
fn main() {
    // 数字代表限定最小宽度，默认会用空格补齐右对齐
    println!("{:5e}", 0.01); // => " 1e-2"
    println!("{:2E}", 1000000000); // => 1E9
}
```

##### 14.5.5 指针地址

```rust
let v= vec![1, 2, 3];
println!("{:p}", v.as_ptr()) // => 0x600002324050
```

##### 14.5.6 转义

```rust
fn main() {
    // "{{" 转义为 '{'   "}}" 转义为 '}'   "\"" 转义为 '"'
    // => Hello "{World}" 
    println!(" Hello \"{{World}}\" ");

    // 下面代码会报错，因为占位符{}只有一个右括号}，左括号被转义成字符串的内容
    // println!(" {{ Hello } ");
    // 也不可使用 '\' 来转义 "{}"
    // println!(" \{ Hello \} ")
}
```

#### 14.6 在格式化字符串时捕获环境中的值(Rust 1.58之后)

```rust
fn get_person() -> String {
    String::from("sunface")
}
fn main() {
    let person = get_person();
    println!("Hello, {person}!");
}
```

将环境中的值用于格式化参数:

```rust
let (width, precision) = get_format();
for (name, score) in get_scores() {
  println!("{name}: {score:width$.precision$}");
}
```

但也有局限，它只能捕获普通的变量，对于更复杂的类型（例如表达式），可以先将它赋值给一个变量或使用以前的 `name = expression` 形式的格式化参数。 目前除了 `panic!` 外，其它接收格式化参数的宏，都可以使用新的特性。对于 `panic!` 而言，如果还在使用 `2015版本` 或 `2018版本`，那 `panic!("{ident}")` 依然会被当成 正常的字符串来处理，同时编译器会给予 `warn` 提示。而对于 `2021版本` ，则可以正常使用:

```rust
fn get_person() -> String {
    String::from("sunface")
}
fn main() {
    let person = get_person();
    panic!("Hello, {person}!");
}
```

输出:

```bash
thread 'main' panicked at 'Hello, sunface!', src/main.rs:6:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

---

### ♾️.一些比较有意思的题目

```rust
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

fn check_size<T>(val: T)
// 此处会检查 where 后面的表达式，即Assert<xxx>是否实现了 IsTrue 特征，而最下面的代码IsTrue只为Assert<true>实现了特征，也就是说，当内部表达式为false时，编译器会报错也即size_of<T> >= 768时，会报错
// 由最下方Assert的枚举实现代码可知，Assert 接收一个常量 bool 值
where
    Assert<{ core::mem::size_of::<T>() < 768 }>: IsTrue,
{
    //...
}

// fix the errors in main
fn main() {
    check_size([0u8; 767]); 
    check_size([0i32; 191]);
    check_size(["hello你好"; 47]); // &str is a string reference, containing a pointer and string length in it, so it takes two word long, in x86-64, 1 word = 8 bytes
    check_size([(); 31].map(|_| "hello你好".to_string()));  // String is a smart pointer struct, it has three fields: pointer, length and capacity, each takes 8 bytes
    check_size(['中'; 191]); // A char takes 4 bytes in Rust
}

pub enum Assert<const CHECK: bool> {}

pub trait IsTrue {}

impl IsTrue for Assert<true> {}
```

---

## 一、进阶

### 1.闭包 Closure

这里首先介绍一下匿名函数的概念。

#### 1.1 匿名函数

即 Anonymous Function，是一种没有名称的函数，通常用于定义一次性或者短期使用的函数。在Rust中的体现就是闭包。就比方说在一个编程语言中，直接通过不写函数名或者类似在`javascript`中使用同一个名称`function`，然后带有一个返回表达式以及函数签名的式子，或许用下面Rust的闭包实例来看会比较好理解。

#### 1.2 闭包

**闭包**最早由上世纪60年代的`Scheme`语言引进，然后后面的大多数语言也渐渐把闭包作为核心特性了。

```rust
fn main() {
    let x = 1;
    let sum = |y| x + y;
    // "||" 内部就是就是要接受的参数值，后面的表达式就是要返回的表达式
    
    assert_eq!(3, sum(2));
}
```



### Arc

`Arc`：用于在多个线程间共享数据，确保数据的引用计数是线程安全的，但不提供修改数据的能力。

### Mutex

`Mutex`：用于确保多个线程对同一数据的访问是互斥的，确保同一时刻只有一个线程能修改数据。

### `RwLock` vs `Mutex` 的区别

1. **锁的类型**：
   - `Mutex`：是 **互斥锁**，它在任何时刻只允许一个线程访问数据，不管是读取还是写入。如果一个线程持有 `Mutex`，其他线程就无法访问该资源，直到锁被释放。
   - `RwLock`：是 **读写锁**，它允许**多个线程同时读取**数据，但是**写操作**是互斥的，即同一时刻只能有一个线程进行写操作，且写操作不能与任何读操作并发执行。
2. **适用场景**：
   - `Mutex`：适用于只有少量线程需要修改共享数据，或者对数据的修改较为频繁的情况。
   - `RwLock`：适用于数据的**读取远多于写入**的场景，能够提供更高的并发性，允许多个线程同时读取，而不会阻塞读线程。只有当一个线程需要修改数据时，`RwLock` 会阻塞其他的读线程和写线程，确保写操作的独占性。
3. **锁的粒度**：
   - `Mutex`：锁粒度较粗，所有对资源的访问（读取和写入）都必须经过 `Mutex`，保证互斥性。
   - `RwLock`：支持两种锁：**读锁**（`R`）和**写锁**（`W`）。多个线程可以同时获得读锁，但只有一个线程可以获得写锁，并且写锁会阻止所有读锁和其他写锁。

### ⚠TODO！
