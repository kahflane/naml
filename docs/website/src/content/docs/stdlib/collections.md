---
title: "std::collections"
description: Array and map operations with functional programming support
---

Comprehensive collection operations for arrays and maps, including functional programming utilities.

## Import

```naml
use std::collections::*;
use std::collections::arrays::*;  // Arrays only
use std::collections::maps::*;    // Maps only
```

## Array Functions

### Access & Inspection

#### count

Get the number of elements in an array.

```naml
fn count<T>(arr: [T]) -> int
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var len: int = count(nums);  // 5
```

#### first

Get the first element.

```naml
fn first<T>(arr: [T]) -> option<T>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
var f: int = first(nums)!;  // 1
```

#### last

Get the last element.

```naml
fn last<T>(arr: [T]) -> option<T>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
var l: int = last(nums)!;  // 3
```

#### get

Get element at index, returns option.

```naml
fn get<T>(arr: [T], index: int) -> option<T>
```

**Example:**

```naml
var nums: [int] = [10, 20, 30];
var val: int = get(nums, 1) ?? 0;  // 20
```

#### reserved

Get the capacity of the array.

```naml
fn reserved<T>(arr: [T]) -> int
```

### Modification

#### push

Append an element to the end.

```naml
fn push<T>(arr: [T], value: T)
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
push(nums, 4);  // [1, 2, 3, 4]
```

#### pop

Remove and return the last element.

```naml
fn pop<T>(arr: [T]) -> option<T>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
var last: int = pop(nums)!;  // 3, array is now [1, 2]
```

#### shift

Remove and return the first element.

```naml
fn shift<T>(arr: [T]) -> option<T>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
var first: int = shift(nums)!;  // 1, array is now [2, 3]
```

#### insert

Insert an element at a specific index.

```naml
fn insert<T>(arr: [T], index: int, value: T)
```

**Example:**

```naml
var nums: [int] = [1, 3, 4];
insert(nums, 1, 2);  // [1, 2, 3, 4]
```

#### remove_at

Remove element at index.

```naml
fn remove_at<T>(arr: [T], index: int)
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4];
remove_at(nums, 1);  // [1, 3, 4]
```

#### remove

Remove first occurrence of a value.

```naml
fn remove<T>(arr: [T], value: T) -> bool
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 2];
remove(nums, 2);  // [1, 3, 2], returns true
```

#### swap

Swap two elements at given indices.

```naml
fn swap<T>(arr: [T], i: int, j: int)
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
swap(nums, 0, 2);  // [3, 2, 1]
```

#### fill

Fill array with a value.

```naml
fn fill<T>(arr: [T], value: T)
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
fill(nums, 0);  // [0, 0, 0]
```

#### clear

Remove all elements.

```naml
fn clear<T>(arr: [T])
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
clear(nums);  // []
```

### Transformation

#### reversed

Return a reversed copy.

```naml
fn reversed<T>(arr: [T]) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
var rev: [int] = reversed(nums);  // [3, 2, 1]
```

#### sort

Sort array in ascending order.

```naml
fn sort<T>(arr: [T]) -> [T]
```

**Example:**

```naml
var nums: [int] = [3, 1, 4, 1, 5];
var sorted: [int] = sort(nums);  // [1, 1, 3, 4, 5]
```

#### flatten

Flatten nested arrays.

```naml
fn flatten<T>(arr: [[T]]) -> [T]
```

**Example:**

```naml
var nested: [[int]] = [[1, 2], [3, 4], [5]];
var flat: [int] = flatten(nested);  // [1, 2, 3, 4, 5]
```

#### take

Take first n elements.

```naml
fn take<T>(arr: [T], n: int) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var first_three: [int] = take(nums, 3);  // [1, 2, 3]
```

#### drop

Drop first n elements.

```naml
fn drop<T>(arr: [T], n: int) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var rest: [int] = drop(nums, 2);  // [3, 4, 5]
```

#### slice

Extract a slice from start to end.

```naml
fn slice<T>(arr: [T], start: int, end: int) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var middle: [int] = slice(nums, 1, 4);  // [2, 3, 4]
```

#### unique

Remove duplicates.

```naml
fn unique<T>(arr: [T]) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 2, 3, 1, 4];
var uniq: [int] = unique(nums);  // [1, 2, 3, 4]
```

#### compact

Remove zero/empty values.

```naml
fn compact<T>(arr: [T]) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 0, 2, 0, 3];
var result: [int] = compact(nums);  // [1, 2, 3]
```

#### chunk

Split array into chunks of size n.

```naml
fn chunk<T>(arr: [T], size: int) -> [[T]]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var chunks: [[int]] = chunk(nums, 2);  // [[1, 2], [3, 4], [5]]
```

#### shuffle

Randomly shuffle array.

```naml
fn shuffle<T>(arr: [T]) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var shuffled: [int] = shuffle(nums);  // Random order
```

### Set Operations

#### intersect

Get common elements.

```naml
fn intersect<T>(a: [T], b: [T]) -> [T]
```

**Example:**

```naml
var a: [int] = [1, 2, 3, 4];
var b: [int] = [3, 4, 5, 6];
var common: [int] = intersect(a, b);  // [3, 4]
```

#### diff

Get elements in first array but not in second.

```naml
fn diff<T>(a: [T], b: [T]) -> [T]
```

**Example:**

```naml
var a: [int] = [1, 2, 3, 4];
var b: [int] = [3, 4, 5];
var result: [int] = diff(a, b);  // [1, 2]
```

#### union

Combine arrays, removing duplicates.

```naml
fn union<T>(a: [T], b: [T]) -> [T]
```

**Example:**

```naml
var a: [int] = [1, 2, 3];
var b: [int] = [3, 4, 5];
var combined: [int] = union(a, b);  // [1, 2, 3, 4, 5]
```

#### zip

Combine two arrays into pairs.

```naml
fn zip<A, B>(a: [A], b: [B]) -> [[A, B]]
```

**Example:**

```naml
var names: [string] = ["Alice", "Bob"];
var ages: [int] = [30, 25];
var pairs: [[string, int]] = zip(names, ages);
```

#### unzip

Split array of pairs into two arrays.

```naml
fn unzip<A, B>(pairs: [[A, B]]) -> ([A], [B])
```

### Search

#### index_of

Find index of first occurrence.

```naml
fn index_of<T>(arr: [T], value: T) -> option<int>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 2, 1];
var idx: int = index_of(nums, 2)!;  // 1
```

#### last_index_of

Find index of last occurrence.

```naml
fn last_index_of<T>(arr: [T], value: T) -> option<int>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 2, 1];
var idx: int = last_index_of(nums, 2)!;  // 3
```

#### contains

Check if array contains a value.

```naml
fn contains<T>(arr: [T], value: T) -> bool
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var has: bool = contains(nums, 3);  // true
```

### Aggregation

#### sum

Calculate sum of numeric array.

```naml
fn sum(arr: [int]) -> int
fn sum(arr: [float]) -> float
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var total: int = sum(nums);  // 15
```

#### min

Find minimum value.

```naml
fn min<T>(arr: [T]) -> option<T>
```

**Example:**

```naml
var nums: [int] = [3, 1, 4, 1, 5];
var minimum: int = min(nums)!;  // 1
```

#### max

Find maximum value.

```naml
fn max<T>(arr: [T]) -> option<T>
```

**Example:**

```naml
var nums: [int] = [3, 1, 4, 1, 5];
var maximum: int = max(nums)!;  // 5
```

#### sample

Get one random element.

```naml
fn sample<T>(arr: [T]) -> option<T>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var random: int = sample(nums)!;
```

#### sample_n

Get n random elements.

```naml
fn sample_n<T>(arr: [T], n: int) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var random_three: [int] = sample_n(nums, 3);
```

### Higher-Order Functions

#### any

Check if any element matches predicate.

```naml
fn any<T>(arr: [T], predicate: fn(T) -> bool) -> bool
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var has_even: bool = any(nums, fn(x: int) -> bool { return x % 2 == 0; });  // true
```

#### all

Check if all elements match predicate.

```naml
fn all<T>(arr: [T], predicate: fn(T) -> bool) -> bool
```

**Example:**

```naml
var nums: [int] = [2, 4, 6, 8];
var all_even: bool = all(nums, fn(x: int) -> bool { return x % 2 == 0; });  // true
```

#### count_if

Count elements matching predicate.

```naml
fn count_if<T>(arr: [T], predicate: fn(T) -> bool) -> int
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var evens: int = count_if(nums, fn(x: int) -> bool { return x % 2 == 0; });  // 2
```

#### apply (map)

Transform each element.

```naml
fn apply<T, U>(arr: [T], mapper: fn(T) -> U) -> [U]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
var doubled: [int] = apply(nums, fn(x: int) -> int { return x * 2; });  // [2, 4, 6]
```

#### where (filter)

Filter elements matching predicate.

```naml
fn where<T>(arr: [T], predicate: fn(T) -> bool) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var evens: [int] = where(nums, fn(x: int) -> bool { return x % 2 == 0; });  // [2, 4]
```

#### reject

Filter elements NOT matching predicate.

```naml
fn reject<T>(arr: [T], predicate: fn(T) -> bool) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var odds: [int] = reject(nums, fn(x: int) -> bool { return x % 2 == 0; });  // [1, 3, 5]
```

#### partition

Split array based on predicate.

```naml
fn partition<T>(arr: [T], predicate: fn(T) -> bool) -> ([T], [T])
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var result: ([int], [int]) = partition(nums, fn(x: int) -> bool { return x % 2 == 0; });
// ([2, 4], [1, 3, 5])
```

#### take_while

Take elements while predicate is true.

```naml
fn take_while<T>(arr: [T], predicate: fn(T) -> bool) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 1];
var result: [int] = take_while(nums, fn(x: int) -> bool { return x < 4; });  // [1, 2, 3]
```

#### drop_while

Drop elements while predicate is true.

```naml
fn drop_while<T>(arr: [T], predicate: fn(T) -> bool) -> [T]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var result: [int] = drop_while(nums, fn(x: int) -> bool { return x < 3; });  // [3, 4, 5]
```

#### flat_apply (flatMap)

Map and flatten nested results.

```naml
fn flat_apply<T, U>(arr: [T], mapper: fn(T) -> [U]) -> [U]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3];
var result: [int] = flat_apply(nums, fn(x: int) -> [int] { return [x, x * 2]; });
// [1, 2, 2, 4, 3, 6]
```

#### find

Find first element matching predicate.

```naml
fn find<T>(arr: [T], predicate: fn(T) -> bool) -> option<T>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var found: int = find(nums, fn(x: int) -> bool { return x > 3; })!;  // 4
```

#### find_index

Find index of first matching element.

```naml
fn find_index<T>(arr: [T], predicate: fn(T) -> bool) -> option<int>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var idx: int = find_index(nums, fn(x: int) -> bool { return x > 3; })!;  // 3
```

#### find_last

Find last element matching predicate.

```naml
fn find_last<T>(arr: [T], predicate: fn(T) -> bool) -> option<T>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var found: int = find_last(nums, fn(x: int) -> bool { return x < 4; })!;  // 3
```

#### find_last_index

Find index of last matching element.

```naml
fn find_last_index<T>(arr: [T], predicate: fn(T) -> bool) -> option<int>
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4, 5];
var idx: int = find_last_index(nums, fn(x: int) -> bool { return x < 4; })!;  // 2
```

#### fold (reduce)

Reduce array to single value.

```naml
fn fold<T, U>(arr: [T], initial: U, reducer: fn(U, T) -> U) -> U
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4];
var product: int = fold(nums, 1, fn(acc: int, x: int) -> int { return acc * x; });  // 24
```

#### scan

Like fold but returns intermediate results.

```naml
fn scan<T, U>(arr: [T], initial: U, reducer: fn(U, T) -> U) -> [U]
```

**Example:**

```naml
var nums: [int] = [1, 2, 3, 4];
var running_sum: [int] = scan(nums, 0, fn(acc: int, x: int) -> int { return acc + x; });
// [1, 3, 6, 10]
```

#### sort_by

Sort with custom comparator.

```naml
fn sort_by<T>(arr: [T], comparator: fn(T, T) -> int) -> [T]
```

**Example:**

```naml
var nums: [int] = [5, 2, 8, 1, 9];
// Descending order
var sorted: [int] = sort_by(nums, fn(a: int, b: int) -> int { return b - a; });
// [9, 8, 5, 2, 1]
```

## Map Functions

### count

Get number of key-value pairs.

```naml
fn count<K, V>(m: map<K, V>) -> int
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
ages["Bob"] = 25;
var size: int = count(ages);  // 2
```

### contains_key

Check if map contains a key.

```naml
fn contains_key<K, V>(m: map<K, V>, key: K) -> bool
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
var has: bool = contains_key(ages, "Alice");  // true
```

### remove

Remove a key-value pair.

```naml
fn remove<K, V>(m: map<K, V>, key: K)
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
remove(ages, "Alice");
```

### clear

Remove all key-value pairs.

```naml
fn clear<K, V>(m: map<K, V>)
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
clear(ages);  // Empty map
```

### keys

Get all keys as an array.

```naml
fn keys<K, V>(m: map<K, V>) -> [K]
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
ages["Bob"] = 25;
var names: [string] = keys(ages);  // ["Alice", "Bob"]
```

### values

Get all values as an array.

```naml
fn values<K, V>(m: map<K, V>) -> [V]
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
ages["Bob"] = 25;
var age_list: [int] = values(ages);  // [30, 25]
```

### entries

Get all key-value pairs.

```naml
fn entries<K, V>(m: map<K, V>) -> [[K, V]]
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
var pairs: [[string, int]] = entries(ages);
```

### first_key

Get first key in iteration order.

```naml
fn first_key<K, V>(m: map<K, V>) -> option<K>
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
var key: string = first_key(ages)!;  // "Alice"
```

### first_value

Get first value in iteration order.

```naml
fn first_value<K, V>(m: map<K, V>) -> option<V>
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
var value: int = first_value(ages)!;  // 30
```

### Higher-Order Map Functions

#### any

Check if any value matches predicate.

```naml
fn any<K, V>(m: map<K, V>, predicate: fn(K, V) -> bool) -> bool
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
ages["Bob"] = 25;
var has_over_25: bool = any(ages, fn(k: string, v: int) -> bool { return v > 25; });  // true
```

#### all

Check if all values match predicate.

```naml
fn all<K, V>(m: map<K, V>, predicate: fn(K, V) -> bool) -> bool
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
ages["Bob"] = 25;
var all_adult: bool = all(ages, fn(k: string, v: int) -> bool { return v >= 18; });  // true
```

#### count_if

Count pairs matching predicate.

```naml
fn count_if<K, V>(m: map<K, V>, predicate: fn(K, V) -> bool) -> int
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
ages["Bob"] = 25;
var count: int = count_if(ages, fn(k: string, v: int) -> bool { return v > 25; });  // 1
```

#### fold

Reduce map to single value.

```naml
fn fold<K, V, U>(m: map<K, V>, initial: U, reducer: fn(U, K, V) -> U) -> U
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
ages["Bob"] = 25;
var total: int = fold(ages, 0, fn(acc: int, k: string, v: int) -> int { return acc + v; });  // 55
```

#### transform

Transform values.

```naml
fn transform<K, V, U>(m: map<K, V>, mapper: fn(K, V) -> U) -> map<K, U>
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
var adult: map<string, bool> = transform(ages, fn(k: string, v: int) -> bool { return v >= 18; });
```

#### where

Filter map by predicate.

```naml
fn where<K, V>(m: map<K, V>, predicate: fn(K, V) -> bool) -> map<K, V>
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
ages["Bob"] = 25;
var over_25: map<string, int> = where(ages, fn(k: string, v: int) -> bool { return v > 25; });
```

#### reject

Filter map by inverted predicate.

```naml
fn reject<K, V>(m: map<K, V>, predicate: fn(K, V) -> bool) -> map<K, V>
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
ages["Bob"] = 25;
var under_30: map<string, int> = reject(ages, fn(k: string, v: int) -> bool { return v >= 30; });
```

### Map Operations

#### merge

Merge two maps (second wins on conflicts).

```naml
fn merge<K, V>(a: map<K, V>, b: map<K, V>) -> map<K, V>
```

**Example:**

```naml
var m1: map<string, int>;
m1["a"] = 1;
var m2: map<string, int>;
m2["b"] = 2;
var merged: map<string, int> = merge(m1, m2);  // {"a": 1, "b": 2}
```

#### defaults

Fill in missing keys from defaults map.

```naml
fn defaults<K, V>(m: map<K, V>, defaults: map<K, V>) -> map<K, V>
```

**Example:**

```naml
var config: map<string, int>;
config["timeout"] = 5000;
var def: map<string, int>;
def["timeout"] = 3000;
def["retries"] = 3;
var final: map<string, int> = defaults(config, def);  // {"timeout": 5000, "retries": 3}
```

#### intersect

Get common keys.

```naml
fn intersect<K, V>(a: map<K, V>, b: map<K, V>) -> map<K, V>
```

**Example:**

```naml
var m1: map<string, int>;
m1["a"] = 1;
m1["b"] = 2;
var m2: map<string, int>;
m2["b"] = 20;
var common: map<string, int> = intersect(m1, m2);  // {"b": 2}
```

#### diff

Get keys in first map but not in second.

```naml
fn diff<K, V>(a: map<K, V>, b: map<K, V>) -> map<K, V>
```

**Example:**

```naml
var m1: map<string, int>;
m1["a"] = 1;
m1["b"] = 2;
var m2: map<string, int>;
m2["b"] = 20;
var result: map<string, int> = diff(m1, m2);  // {"a": 1}
```

#### invert

Swap keys and values.

```naml
fn invert<K, V>(m: map<K, V>) -> map<V, K>
```

**Example:**

```naml
var ages: map<string, int>;
ages["Alice"] = 30;
var inverted: map<int, string> = invert(ages);  // {30: "Alice"}
```

#### from_arrays

Create map from key and value arrays.

```naml
fn from_arrays<K, V>(keys: [K], values: [V]) -> map<K, V>
```

**Example:**

```naml
var names: [string] = ["Alice", "Bob"];
var ages: [int] = [30, 25];
var age_map: map<string, int> = from_arrays(names, ages);
```

#### from_entries

Create map from array of pairs.

```naml
fn from_entries<K, V>(entries: [[K, V]]) -> map<K, V>
```

**Example:**

```naml
var pairs: [[string, int]] = [["Alice", 30], ["Bob", 25]];
var age_map: map<string, int> = from_entries(pairs);
```
