# MM of NPUcore

已经处理好的内容&不需要处理的内容

- [x] address
- [x] frame_allocator
- [x] heap_allocator
- [ ] map_area
- [ ] memory_set
- [x] page_table
- [x] zram
- [x] mod

## 一、address

待处理：RV中的 VirtPageNum的方法indexes使用的是3级页表，而LA版本中这个数值可以变化（当然实际使用的是后，这个参数设定为**3**）。

| 类型名称            | 作用                               | 方法                                                         |
| ------------------- | ---------------------------------- | ------------------------------------------------------------ |
| PhysAddr            | 物理地址                           | floor, ceil, page_offset, aligned                            |
| VirtAddr            | 虚拟地址                           | floor, ceil, page_offset, aligned                            |
| PhysPageNum         | 物理页号                           | start_addr, offset, get_pte_array, get_bytes_array, get_dwords_array, get_mut |
| VirtPageNum         | 虚拟页号                           | start_addr, offset, indexes                                  |
| SimpleRange         | 表示一个范围区间                   | new, get_start, get_end                                      |
| SimpleRangeIterator | 范围迭代器                         | \                                                            |
| VPNRange            | 包裹VPN的上述SimpleRange的类型别名 | \                                                            |
| PPNRange            | 包裹PPN的上述SimpleRange的类型别名 | \                                                            |

| 特征名称  | 作用             | 方法 |
| --------- | ---------------- | ---- |
| StepByOne | 提供单步递增能力 | step |

| 宏名称                 | 作用                       |
| ---------------------- | -------------------------- |
| show_frame_consumption | 用于测量代码块的帧消耗情况 |

## 二、frame_allocator

| 类型名称            | 作用                                     | 方法                                                 |
| ------------------- | ---------------------------------------- | ---------------------------------------------------- |
| FrameTracker        | 物理帧跟踪器，跟踪一个物理页面的生命周期 | new, new_uninit                                      |
| StackFrameAllocator | 栈式帧分配器                             | clear, init, unallocated_frames, new, alloc, dealloc |
| FrameAllocatorImpl  | StackFrameAllocator的别名                | 同上                                                 |

| 特征名称       | 作用                                                 | 方法                |
| -------------- | ---------------------------------------------------- | ------------------- |
| FrameAllocator | 帧分配器的接口（后续可以将栈式帧分配器换成别的类型） | new, alloc, dealloc |

| 全局变量        | 类型               | 作用         | 相关函数                                                     |
| --------------- | ------------------ | ------------ | ------------------------------------------------------------ |
| FRAME_ALLOCATOR | FrameAllocatorImpl | 全局帧分配器 | init_frame_allocator, oom_handler, frame_reserve, frame_alloc, frame_dealloc, unallocated_frames |

## 三、heap_allocator

| 全局变量           | 类型                   | 作用 |
| ------------------ | ---------------------- | ---- |
| **HEAP_ALLOCATOR** | LockedHeap<32>         |      |
| **HEAP_SPACE**     | [u8; KERNEL_HEAP_SIZE] |      |

| 全局变量       | 类型           | 作用         | 相关函数             |
| -------------- | -------------- | ------------ | -------------------- |
| HEAP_ALLOCATOR | LockedHeap<32> | 全局堆分配器 | init_heap, heap_test |

## 四、map_area

主要需要处理PageTable以及MapPermission相关的内容

| 类型名称      | 作用 | 方法                                                         |
| ------------- | ---- | ------------------------------------------------------------ |
| Frame         |      | insert_in_memory, take_in_memory, **gen_id(unused)**, swap_out, force_swap_out, swap_in, zip, unzip |
| LinearMap     |      | **gen_dict(unused)**, **get_start(unused)**, **get_end(unused)**, new, get_mut, get_in_memory, alloc_in_memory, remove_in_memory, set_start, set_end, into_two, into_three,  count_compressed_and_swapped, split_activate_into_two, split_activate_into_three |
| MapArea       |      | new, from_another, from_existing_frame, map_one, map_one_unchecked, **map_one_zeroed_unchecked**, **unmap_one**, map_from_existing_page_table, **get_inner**, **get_start**, **get_end**, **get_lock(unused)**, map_from_kernel_area, unmap, copy_on_write, expand_to, shrink_to, rshrink_to, check_overlapping, **into_two**, **into_three**, do_oom, force_swap |
| MapType       |      |                                                              |
| MapPermission |      |                                                              |
| MapFlags      |      |                                                              |

## 五、memory_set

| 类型名称                | 作用                             | 方法                                                         |
| ----------------------- | -------------------------------- | ------------------------------------------------------------ |
| MemoryError             | 枚举类型，包含各种类型的内存错误 |                                                              |
| MemorySet<T: PageTable> |                                  | new_bare, **new_bare_kern**, token, insert_framed_area, insert_program_area, remove_area_with_start_vpn, push, push_with_offset, **get_area_by_vpn_range**, push_no_alloc, last_mmap_area_idx, highest_addr, contains_valid_buffer, do_page_fault, do_shallow_clean, do_deep_clean, map_trampoline, map_signaltrampoline, new_kernel, map_elf, from_elf, from_existing_user, activate, translate, set_pte_flags, clear_access_bit, clear_dirty_bit, recycle_data_pages, show_areas, sbrk, mmap, munmap, mprotect, create_elf_tables, alloc_user_res, dealloc_user_res, **is_dirty** |

| 全局变量     | 类型                                                         | 作用 | 相关函数 |
| ------------ | ------------------------------------------------------------ | ---- | -------- |
| KERNEL_SPACE | Arc\<Mutex\<MemorySet\<None or crate::mm::KernelPageTableImpl>>> |      |          |

| 其他函数         | 作用 |
| ---------------- | ---- |
| check_page_fault |      |
| remap_test       |      |

## 六、page_table

## 七、zram

| 类型名称    | 作用                             | 方法                                           |
| ----------- | -------------------------------- | ---------------------------------------------- |
| ZramError   | 定义了Zram操作可能遇到的错误类型 | \                                              |
| ZramTracker | Zram的跟踪器，包装了一个Zram索引 | \                                              |
| Zram        | Zram设备类型                     | new, insert, get, remove, read, write, discard |

| 全局变量    | 类型                  | 作用         | 相关函数 |
| ----------- | --------------------- | ------------ | -------- |
| ZRAM_DEVICE | **Arc<Mutex\<Zram>>** | 全局ZRAM设备 | 同Zram   |
