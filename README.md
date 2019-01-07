# nail

Nail is a programming language I wrote to learn about writing programming languages. It is not production ready, but it was an instructive and succesful project.

Nail was originally based on Lox, the language presented in [Crafting Interpreters](http://craftinginterpreters.com/) by Bob Nystrom, rewritten in Rust, but it has diverged substantially and is mostly unrecognisable.

The Nail compiler and virtual machine are implemented in Rust. It uses a handwritten recursive descent parser, parses into an AST, and then produces bytecode directly. There are currently no optimisations performed.

Nail is mostly feature complete, although it is notably lacking any kind of garbage collection.

There's a decent body of example Nail code in [my Advent of Code 2018 solutions](https://github.com/m-r-hunt/aoc2018). I also wrote a bit about my experience writing Nail [on my blog](http://mechtoast.com/blog/languages/adventures-in-programming-language-design/).
