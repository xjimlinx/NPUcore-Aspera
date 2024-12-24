---
title: Makefile小记
author: Xein
categories: 
    - 编程语言
    - Makefile
updated: 2024-11-6 01:13
date: 2024-11-5 23:15
tag:
    - Makefile
    - 编程
    - 基础
description: Makefile 相关笔记
---
# Makefile小记

## 赋值语句

### 1. `=`

| 符号 | 含义                                   |
| ---- | -------------------------------------- |
| =    | 变量的值是整个makefile中最后被指定的值 |

**例子**

```makefile
VIR_A = A
VIR_B = $(VIR_A) B
VIR_A = AA
```

最终 `VIR_B` 的值是"AA B" 而不是 "A B"

在make时，会把整个makefile展开，然后决定最后变量的值

### 2. `:=`

| 符号 | 含义     |
| ---- | -------- |
| :=   | 直接赋值 |

**例子**

```makefile
VIR_A = A
VIR_B = $(VIR_A) B
VIR_A = AA
```

最终 `VIR_B` 的值是"A B"

### 3. `?=`

| 符号 | 含义                                                 |
| ---- | ---------------------------------------------------- |
| ?=   | 如果变量没有被赋值，则赋予等号后面的值，相当于默认值 |

**例子**

```makefile
# Makefile1
VIR ?= new_value
# 上面就会等于 new_value
```

```makefile
# Makefile2
VIR := old_value
VIR ?= new_value
# 上面就会等于 old_value
```

### 4. `+=`

| 符号 | 含义                                             |
| ---- | ------------------------------------------------ |
| +=   | 将等号后面的值添加到前面的变量上，中间会插入空格 |

**例子**

```makefile
varA := a
varA += 1
debug:
	@echo $(varA)
# 输出
# a a
```
