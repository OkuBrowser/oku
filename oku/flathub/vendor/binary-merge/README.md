# Minimum comparison merge of two sorted random acces sequences

## Problem

At the end of 2014, I was thinking about what would be the most efficient way to merge two sorted sequences.

The answer is obviously trivial if you consider *copying* elements to be roughly as expensive as *comparing* elements. In that case, simply compare the *first remaining element* of each sequence and take the smaller one, until you run out of elements in one of the sequences, then just copy the rest. 

But in many cases this the assumption that comparing is as expensive as copying is not true. Let's say you have a sequence of `BigInt`, `Rational`, very long `String` or complex tuples. In that case *comparing* two elements will be *several orders of magnitude* more expensive than *copying* an element.

So let's consider the case where only the number of comparisons matters, and the copying is considered to be essentially free  (Copying a pointer is just about the cheapest operation you can do. You can literally copy millions of pointers in less than a millisecond on a modern machine).

In that case, the seemingly trivial problem of merging two sorted lists turns into a problem that has *10 pages of [TAOCP](https://en.wikipedia.org/wiki/The_Art_of_Computer_Programming)* devoted to it (Volume 3, Pages 197-207, **Minimum-Comparison Merging**)

So I did what you usually do in this situation: [ask on stackexchange](http://programmers.stackexchange.com/questions/267406/algorithm-to-merge-two-sorted-arrays-with-minimum-number-of-comparisons). Given that this should be a pretty common problem, I was expecting an answer like "you obviously have to use the Foo-Bar algorithm described in 1969 by XYZ". But to my surprise, the algorithm that was posted as the answer, despite being called [A simple algorithm for merging two disjoint linearly-ordered sets (F. K. Hwang , S. Lin)](http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.419.8292), is not very simple. It is asymptotically optimal, but too complex to degrade well in the case that the comparison is relatively cheap. Also, it is pretty complex to implement. 

So I tried to come up with a simpler solution.

## Cases

There are several cases that have to be considered when merging two sorted sequences. Coming up with a good solution for any of these cases is simple. The challenge is to come up with a solution that works well for **all** of the cases and that gracefully degrades in the worst case.

a) Merging long sequence with single element sequence

```js
a = [1,2,3,4,6,7,8,9,10]
b = [5]
```

The best solution in this case is to do a binary search for the insertion point of the single element of `b` in `a`, then just copy 

- the part of `a` that is below `b[0]`
- the element `b[0]`
- the part of `a` that is above `b[0]`

Obviously it would be possible to just special-case this solution. But that would be unelegant and in any case would not help in case b)

b) Merging a long sequence and a short sequence

```js
a = [1,2,4,5,6,7,9,10]
b = [3,8]
```

In this case you might be tempted to just insert all elements of the smaller list into the larger list, doing binary searches for each insert. But that would be less than optimal. From the insertion position of the first element, we know which elements are definitely smaller than the second element and thus do not have to be compared, so we can restrict the range of the second binary search based on the result of the first.

c) Merging two large sequences which are non-overlapping

```js
a = [1,2,3,4,5]
b = [6,7,8,9,10]
```

This is a case where you can expect huge performance gains, because you just have to copy one list after the other. You could detect this case by comparing the first element of one sequence with the last element of the other sequence and vice versa. But the cost of that comparison will be overhead in other cases, so you can only justify this if you know that this case is very common (which we don't).

d) Merging two completely interleaved sequences

```js
a = [1,3,5,7,9]
b = [2,4,6,8,10]
```

This is the worst case, where it won't be possible to get better results than the linear merge. Any good algorithm should gracefully handle this case without doing much more than m + n - 1 comparisons. Depending on what you expect as the average case, doing twice as many comparisons might still be OK. But e.g. *o(m log n)* comparisons, like you would get by inserting all *n* elements from the smaller list into the larger list with *m* elements, would *not* be ok.

## Coming up with a good algorithm

Being a functional programmer, I think that the most elegant algorithms are recursive. So let's think about how a recursion step would look like.

### Naming

Let's use `a0` and `a1` for the first (inclusive) and last (exclusive) index of `a` that we're currently interested in. Likewise, `b0` and `b1` for the first (inclusive) and last (exclusive) index of `b` that we're currently interested in.

### The base cases

Before we start thinking about complex things, let's consider the base case(s). Merging a section of a sequence with an *empty* section of another sequence means just copying over all elements of interest from that sequence to the target sequence. So if `a0` is `a1`, just copy everything from `b0` until `b1` to the result, and vice versa.

### The first comparison

It is clear that we have to gain the maximum information from each comparison in order to limit the number of comparisons to the minimum. So it seems intuitively obvious that we have to compare the *middle* element of `a` with the *middle* element of `b`. No matter what the result of the comparison is, we have 50% of all elements in `a` that we never again have to compare with 50% of the elements in `b`. We have gained information for a quarter of all possible comparisons with just a single comparison. If you had a table of size m \* n with each cell being a possible comparison, executing the comparison at the *center* of the table allows you to eliminate an entire quadrant of the table.

|   | 5 | 6 | 7 | 8 | 9 |
|---|---|---|---|---|---|
| 1 |   |   | > | > | > |
| 3 |   |   | > | > | > |
| 5 |   |   | > | > | > |
| 7 |   |   |   |   |   |
| 9 |   |   |   |   |   |

```
am = (a0 + a1) / 2
bm = (b0 + b1) / 2
````

`a(am) < b(bm)`, so *all* elements `a[i], a0 ≤ i ≤ am` are smaller than *all* elements `b[j], bm ≤ j < b1`.

### The recursion step

Now that know what we have to do for the first comparison, what do we do with it? What I came up with is the following: we look for the *insertion index* of the center element of `a` in `b`, using a binary search. The first comparison done by the binary search will be exactly as described above. Once we have the result, which we shall call `bm`, we can recurse.

We have to merge elements `a0 until am` from `a` with all elements `b0 until bm` from `b`. Then we have to copy the single element `a(am)` to the result, and finally merge elements `am + 1 until a1` from `a` with all elements `bm + 1 until b1` from `b`.

And that's it. Here is our code, for the case that `a` and `b` are disjoint ordered sets.

```rust
    fn binary_merge(&self, m: &mut M, an: usize, bn: usize) -> bool {
        if an == 0 {
            bn == 0 || self.from_b(m, bn)
        } else if bn == 0 {
            an == 0 || self.from_a(m, an)
        } else {
            // neither a nor b are 0
            let am: usize = an / 2;
            // pick the center element of a and find the corresponding one in b using binary search
            let a = &m.a_slice()[am];
            match m.b_slice()[..bn].binary_search_by(|b| self.cmp(a, b).reverse()) {
                Ok(bm) => {
                    // same elements. bm is the index corresponding to am
                    // merge everything below am with everything below the found element bm
                    self.binary_merge(m, am, bm) &&
                    // add the elements a(am) and b(bm)
                    self.collision(m) &&
                    // merge everything above a(am) with everything above the found element
                    self.binary_merge(m, an - am - 1, bn - bm - 1)
                }
                Err(bi) => {
                    // not found. bi is the insertion point
                    // merge everything below a(am) with everything below the found insertion point bi
                    self.binary_merge(m, am, bi) &&
                    // add a(am)
                    self.from_a(m, 1) &&
                    // everything above a(am) with everything above the found insertion point
                    self.binary_merge(m, an - am - 1, bn - bi)
                }
            }
        }
    }
```

Note that while this method is using recursion, it is not referentially transparent. The result sequence is built in the methods fromA and fromB using a mutable builder for efficiency. Of course, you will typically wrap this algorithm in a referentially transparent way.

Also note that the [version in spire](https://github.com/rklaehn/spire/blob/eb70e8e89f669c1cdb731cacf5398c4f9e0dd3f7/core/shared/src/main/scala/spire/math/Merging.scala#L61) is slightly more complex, because it also works for the case where there are common elements in `a` and `b`, and because it is sometimes an advantage to have the insertion point.

Here is an [example](https://github.com/rklaehn/spire/blob/eb70e8e89f669c1cdb731cacf5398c4f9e0dd3f7/core/shared/src/main/scala/spire/math/Merging.scala#L101) how the merging strategy is used to merge two sorted `Array[T]` given an `Order[T]`.

## Behavior for the cases described above

a) Merging long list with single element list

It might seem that the algorithm is not symmetric. But at least for the case of merging a large list with a single element list, the algorithm boils down to a binary search in both cases.

b) Merging a long list and a small list

The algorithm will use the information from the comparison of both middle elements to avoid unnecessary comparisons

c) Merging two long non-overlapping lists

The algorithm will figure out in O(log n) in the first recursion step that the lists are disjoint, and then just copy them

d) Merging interleaved lists

This is tricky, but tests with counting comparisons have indicated that the maximum number of comparisons is never much more than `m + n - 1`.
