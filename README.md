# A macro frontend for oxide-enzyme

Oxide-enzyme is the rust frontend of Enzyme, a tool which can differentiate your Rust functions `f(x,y)` in the calculus sense.
This autodiff crate will not differentiate your code directly, but rather create some function declarations `d_f` which will later be filled by oxide-enzyme.
When using the same settings for the differentiate\_ext macro and the build.rs file from oxide-enzyme function signatures are guaranteed to match.  

It is possible to use oxide-enzyme without this frontend by writing function declarations manually, although not recommended.
Writing function declarations which do not match Enzymes expectation is not guaranteed to be catched as a compile time error and can just lead to incorrect gradients.  

It is possible to differentiate the same function multiple times by adding multiple macros with different settings.
