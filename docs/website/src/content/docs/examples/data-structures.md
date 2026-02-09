---
title: Data Structures
description: Arrays, collections, and tree implementations in naml
---

## Collections Demo

Using the standard library's collection functions with lambdas:

```naml
use std::collections::arrays::*;

fn main() {
    var nums: [int] = [3, 1, 4, 1, 5, 9, 2, 6];
    println("Input: {}", nums);

    // Access
    println("first: {}", first(nums)!);
    println("last: {}", last(nums)!);

    // Aggregation
    println("sum: {}", sum(nums));
    println("min: {}", min(nums)!);
    println("max: {}", max(nums)!);

    // Transformation
    println("reversed: {}", reversed(nums));
    println("take(3): {}", take(nums, 3));
    println("drop(3): {}", drop(nums, 3));
    println("slice(2, 5): {}", slice(nums, 2, 5));

    // Search
    println("index_of(5): {}", index_of(nums, 5)!);
    println("contains(5): {}", contains(nums, 5));

    // Lambda-based operations
    println("any > 7: {}", any(nums, fn(x: int) -> bool { return x > 7; }));
    println("all > 0: {}", all(nums, fn(x: int) -> bool { return x > 0; }));
    println("doubled: {}", apply(nums, fn(x: int) -> int { return x * 2; }));
    println("evens: {}", where(nums, fn(x: int) -> bool { return x % 2 == 0; }));
    println("find > 5: {}", find(nums, fn(x: int) -> bool { return x > 5; })!);

    // Fold (reduce)
    var product: int = fold(nums, 1, fn(acc: int, x: int) -> int { return acc * x; });
    println("product: {}", product);

    // Flatten nested arrays
    var nested: [[int]] = [[1, 2], [3, 4], [5, 6]];
    println("flatten: {}", flatten(nested));

    // Sort
    println("sorted: {}", sort([5, 2, 8, 1, 9]));
    println("sorted desc: {}", sort_by([5, 2, 8, 1, 9], fn(a: int, b: int) -> int { return b - a; }));
}
```

## Binary Search Tree

A BST implemented using parallel arrays. Each node stores a value, left child index, and right child index. `-1` represents null.

```naml
use std::collections::arrays::{count, push};

fn insert(vals: [int], lefts: [int], rights: [int], root: int, val: int) -> int {
    var idx: int = count(vals);
    push(vals, val);
    push(lefts, -1);
    push(rights, -1);

    if (root == -1) {
        return idx;
    }

    var cur: int = root;
    var placed: bool = false;
    while (not placed) {
        if (val < vals[cur]!) {
            if (lefts[cur]! == -1) {
                lefts[cur] = idx;
                placed = true;
            } else {
                cur = lefts[cur]!;
            }
        } else {
            if (rights[cur]! == -1) {
                rights[cur] = idx;
                placed = true;
            } else {
                cur = rights[cur]!;
            }
        }
    }
    return root;
}

fn search(vals: [int], lefts: [int], rights: [int], root: int, val: int) -> bool {
    var cur: int = root;
    while (cur != -1) {
        if (val == vals[cur]!) {
            return true;
        }
        if (val < vals[cur]!) {
            cur = lefts[cur]!;
        } else {
            cur = rights[cur]!;
        }
    }
    return false;
}

fn inorder(vals: [int], lefts: [int], rights: [int], node: int) {
    if (node == -1) {
        return;
    }
    inorder(vals, lefts, rights, lefts[node]!);
    print("{} ", vals[node]!);
    inorder(vals, lefts, rights, rights[node]!);
}

fn tree_size(lefts: [int], rights: [int], node: int) -> int {
    if (node == -1) {
        return 0;
    }
    return 1 + tree_size(lefts, rights, lefts[node]!)
             + tree_size(lefts, rights, rights[node]!);
}

fn main() {
    var vals: [int] = [];
    var lefts: [int] = [];
    var rights: [int] = [];
    var root: int = -1;

    // Build tree: 50, 30, 70, 20, 40, 60, 80
    //        50
    //       /  \
    //     30    70
    //    / \   / \
    //  20  40 60  80

    root = insert(vals, lefts, rights, root, 50);
    root = insert(vals, lefts, rights, root, 30);
    root = insert(vals, lefts, rights, root, 70);
    root = insert(vals, lefts, rights, root, 20);
    root = insert(vals, lefts, rights, root, 40);
    root = insert(vals, lefts, rights, root, 60);
    root = insert(vals, lefts, rights, root, 80);

    print("Inorder: ");
    inorder(vals, lefts, rights, root);
    println("");

    println("Size: {}", tree_size(lefts, rights, root));
    println("Search 40: {}", search(vals, lefts, rights, root, 40));
    println("Search 25: {}", search(vals, lefts, rights, root, 25));
}
```
