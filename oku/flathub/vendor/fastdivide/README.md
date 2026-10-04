[![Build Status](https://travis-ci.org/fulmicoton/fastdivide.svg?branch=master)](https://travis-ci.org/fulmicoton/fastdivide)

# FastDivide...

FastDivide is a simple/partial RUST port of [libdivide](https://libdivide.com/) by ridiculous_fish
Libdivide is distributed under the zlib license, so does fast divide.

This port is only partial :
It only include one implementation of division for `u64` integers.

# Yes, but what is it really ?

Division is a very costly operation for your CPU.
You may have noticed that when the divisor is known at compile time, your compiler transforms the operations into a cryptic combination of
a multiplication and bitshift.

The key idea is that, rather than computing 

    N / D

It is faster to compute (with k sufficiently large)

    N * ( 2^k / D ) / (2^k)

If D is known in advance, (2^k / D) can be precomputed by the compiler.

Unfortunately if the divisor is unknown at compile time, the compiler cannot use this trick.

The point of `fastdivide` is to apply the same trick by letting you precompute a `DivideU64` object.


# When is it useful ?

If you do a lot (> 10) of division with the same divisor ; and this division is a bottleneck in your program.

This is for instance useful to compute histograms.
