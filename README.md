# Zordle: ZK Wordle

Zordle is [Wordle](https://www.nytimes.com/games/wordle/index.html), but with zero-knowledge proofs. Zordle uses ZK proofs to prove that someone knows words that map to a certain share, but does not reveal those words to a verifier. Zordle is probably the first end-to-end web app built using [Halo 2](https://github.com/zcash/halo2/) ZK proofs!

This project was made as part of [0xPARC Halo 2 Learning Group](https://0xparc.org/blog/halo2-learning-group). Big shoutout to [Ying Tong](https://twitter.com/therealyingtong) for basically hand-holding me through Halo2 circuit writing and to [Uma](https://twitter.com/pumatheuma) and [Blaine](https://twitter.com/BlaineBublitz) for significant work on porting the Halo 2 library to WASM.

# Demo

## [✨Live at zordle.xyz ✨](https://zordle.xyz/)

(insert video!)

# Motivation and user flow

Earlier this year, Wordle became one of the most popular word games, with millions logging on every day to attempt the day's Wordle and share their successes with friends and social media. Wordle's popularity was primarily driven by a really simple to share grid:

<img src="https://user-images.githubusercontent.com/6984346/178630626-65108409-9fbf-4f08-bca6-66b4fa426fff.png" width="32%" />

_At some point, my only form of communication with some of my friends was Wordle grid exchanges_

However, the ease of sharing of these emoji boxes came with an unfortunate flaw: A player could just edit their grid after the fact and make themselves seem much smarter than they originally were. I was always suspicious if my friends really got the scores they claimed or not. ZK SNARKs to the rescue! 🤓

In Zordle, after solving the day's Wordle, a user additionally generates a ZK proof attesting that they know the set of words that perfectly correspond to a set of emoji boxes that they're sharing![^1]

[^1]: Ignore the minor technical detail that they can always just cheat by looking up the day's word elsewhere. 😅

Learning about the shiny new features of Halo 2, Wordle seemed like the perfect application to get my hands dirty with them, so I chose to work on this as my project! The rest of this README will be a technical note on the circuits and  an informal introduction to the Halo 2 Library and features of PLONKish proving systems.

# Circuit

The over-simplified mental model of Halo 2 I've come to appreciate is that of a giant spreadsheet: You have a cells in a tabular format you can fill values in, mutate them from cell to cell, and check that relationships and constraints you'd desire hold true. Additionally, you have access to some "global" structures that are more powerful than just plain cell relationship comparators: you can check row A is a permutation of row B for a very cheap cost, and its also very cheap to check set membership of the value of a particular cell in a giant list (as long as you can define said giant list at "compile time").

To be slightly more precise, Halo 2 essentially structures circuits as row-column operations: There are 4 primary types of columns:

- Instance columns: these are best associated with public inputs
- Advice columns: these are best associated with private inputs and the computation trace of the circuit, the "witness" and,
- Fixed columns: constants used in the computation, known at "compile time"
- Selector columns: these are binary values used to "select" particular advice and instance cells and define constraints between them.

Additionally, there's the notion of a lookup column that allows you to check set membership efficiently but that's perhaps best thought of as a giant fixed set instead of a circuit table column.

Of course, one of the most natural questions, given this spreadsheet, is to figure what's the right way to write efficient ZK circuits inside this playground? Should I use more rows? Or more columns? The answer is quite complicated. For simpler schemes like Groth16 that are based on R1CS, circuit engineers have universally accepted the metric of "the number of constraints" since most tasks associated with the ZK proof (compilation, proving time, trusted setup compute etc.) scale linearly with the number of non-linear constraints. The structure of PLONK circuits, on the other hand, allows for much more flexibility in defining a circuit, and with it comes a very tough to grasp cost model. There are some rough heuristics however, more rows make proving time slower (notably, however, the big delta in proving times occurs at powers of 2, where the cost of the intermediate polynomial FFTs required cuts by half), while more columns make verification time slower. Halo 2, the library, in fact, is very flexible and allows for instantiation of circuits using different polynomial commitment schemes (such as IPA and KZG) which makes cost-modelling instantiation dependent as well. 

# API

- rotation is local in a global table
- instance cols are global indexed
- region is a local construct

# WASM Port



