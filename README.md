# Zordle: ZK Wordle

Zordle is [Wordle](https://www.nytimes.com/games/wordle/index.html), but with zero-knowledge proofs. Zordle uses ZK proofs to prove that a player knows words that map to their shared grid, but does not reveal those words to a verifier. Zordle is probably the first end-to-end web app built using [Halo 2](https://github.com/zcash/halo2/) ZK proofs!

This project was made as part of [0xPARC Halo 2 Learning Group](https://0xparc.org/blog/halo2-learning-group). Big shoutout to [Ying Tong](https://twitter.com/therealyingtong) for basically hand-holding me through Halo2 circuit writing and to [Uma](https://twitter.com/pumatheuma) and [Blaine](https://twitter.com/BlaineBublitz) for significant work on porting the Halo 2 library to WASM.

# [✨Demo: Live at zordle.xyz ✨](https://zordle.xyz/)

https://user-images.githubusercontent.com/6984346/178832179-f9ae5ca5-e271-49b5-84ba-848fcd970b45.mov


# Motivation and user flow

Earlier this year, Wordle became one of the most popular word games, with millions logging on every day to attempt the day's Wordle and share their successes with friends and social media. Wordle's popularity was primarily driven by a really simple to share grid:

<img src="https://user-images.githubusercontent.com/6984346/178630626-65108409-9fbf-4f08-bca6-66b4fa426fff.png" width="32%" />

_At some point, my only form of communication with some of my friends was Wordle grid exchanges_

However, the ease of sharing these emoji boxes came with an unfortunate flaw: A player could just edit their grid after the game and make themselves seem much smarter than they originally were. I was always suspicious if my friends _really_ got the scores they claimed or not. ZK SNARKs to the rescue! 🤓

In Zordle, after solving the day's Wordle, a user additionally generates a ZK proof attesting that they know the set of words that perfectly correspond to a set of emoji boxes that they're sharing![^1]

[^1]: Ignore the minor technical detail that they can always just cheat by looking up the day's word elsewhere. 😅

Learning about the shiny new features of Halo 2, Wordle seemed like a cool toy application to get my hands dirty with the library, so I chose to work on this as my learning group project! The rest of this README will be a technical note on the circuits and an informal introduction to the Halo 2 Library and features of PLONKish proving systems.

# Circuit

The over-simplified mental model of Halo 2 I've come to appreciate is that of a giant spreadsheet: You have cells in a tabular format you can fill values in, mutate the values from cell to cell, and check that relationships and constraints you'd desire hold. Additionally, you have access to some "global" structures that are more powerful than just plain cell relationship comparators: you can check if row A is a permutation of row B for a very cheap cost, and its also very cheap to check set membership of the value of a particular cell in a giant list (as long as you can define said giant list at "compile time").

<img width="1187" alt="Muse MuseBoard 2022-07-13 11 37 57" src="https://user-images.githubusercontent.com/6984346/178774262-4e87557c-e66d-4d90-b3bd-94100bcafc49.png">


To be slightly more precise, Halo 2 essentially structures circuits as row-column operations. There are 4 primary types of columns:

- Instance columns: these are best associated with public inputs,
- Advice columns: these are best associated with private inputs and the computation trace of the circuit, the "witness",
- Fixed columns: constants used in the computation, known at "compile time" and,
- Selector columns: these are binary values used to "select" particular advice and instance cells and define constraints between them.

Additionally, there's the notion of a lookup column that allows you to check set membership efficiently but that's perhaps best thought of as a giant fixed set instead of a circuit table column.

Of course, the natural question, given this abstraction, is to figure out what's the right way to write efficient ZK circuits inside this playground? Should I use more rows? Or more columns? The answer is quite complicated.

For simpler schemes like [Groth16 that are based on R1CS](https://eprint.iacr.org/2016/260.pdf), circuit engineers have universally accepted the metric of "the number of constraints" since most tasks associated with the ZK proof (compilation, proving time, trusted setup compute etc.) scale linearly with the number of non-linear constraints. The structure of PLONK circuits, on the other hand, allows for much more flexibility in defining a circuit, and with it comes a very tough-to-grasp cost model. There are some rough heuristics. For instance, more rows make proving time slower (notably, however, the big jump in proving times occurs when the number of rows crosses powers of 2, where the time cost of the intermediate polynomial FFTs required doubles) while more columns make verification time slower. Notably, also, Halo 2, the library, is very flexible and allows for the instantiation of circuits using different polynomial commitment schemes (such as [IPA](https://eprint.iacr.org/2020/499.pdf#page=49) and [KZG](https://dankradfeist.de/ethereum/2020/06/16/kate-polynomial-commitments.html)) which makes cost-modelling instantiation dependent as well.

The abstraction of a spreadsheet for PLONKish arithmetisation is quite powerful because it allows the library to lay out and pack the rows and columns of your circuit tighter automatically (and paves the way for an IR/automated circuit optimiser long term). While great for optimizations, unfortunately, this ability to auto-pack comes at the expense of making the API more nuanced and makes the cost modelling of circuits even more non-trivial to a circuit programmer.

To elaborate on the _nuance_ of the API, Halo 2 defines the concept of a "region" inside the spreadsheet. A region is the minimal set of cells such that all constraints relating to any one of them are contained entirely within this region. This is a mouthful, but in essence, regions are the minimal building blocks of a circuit. Typically, even non-ZK apps are written in disparate modules - a good analogy for this is perhaps the Clock app on your mobile phone: the app is structured coherently to a user (around "time") but if you think about it like a programmer, the timer tab has very little in common with the world clock tab. The same is true for ZK circuits, a Clock circuit might want to check both the world clock and the status of a timer, and the representation of each of these is its own "region" in the Clock circuit, independent of each other. In the Halo 2 setup, a programmer will write both of them almost independently, and let the compiler figure out the best way to "pack" them into the spreadsheet.

![image0](https://user-images.githubusercontent.com/6984346/178660515-0d5b74ae-a7e6-4973-b5f3-fb174e305991.jpg)
_The concept of "time" is coherent to a user, but the world clock and the timer are disparate modules to a programmer_

While cost-modelling seems to be quite problematic for circuit writers on the surface with the Halo 2 library set up right now, the general read of participants in the Halo 2 Learning Group seems to be that these APIs give circuit writers enough breathing room to hyper-optimise computation for commonly used primitive circuits, and in future, these efficient primitives can be composed (perhaps inefficiently) into real apps by higher-level circuit writers. Hopefully, eventually, ZK circuits will get to a point where a few low-level programmers will hyper-optimise a minimal instruction set, and the rest of us will just roll our circuits into compilers composing and optimising circuits on those instruction sets.[^2]

[^2]: Supposedly, this will also mark the switching point where we can stop bothering with hyper-optimised zkEVMs and instead just write a zkMIPS machine for all VMs. Some notes on this tradeoff [here](https://kelvinfichter.com/pages/thoughts/hybrid-rollups/).

It's certainly fun to theorise about the future of ZK circuit writing, but right now, we have a very real task at hand: making an anti-cheat that doesn't really work for a meaningless word game that's not even popular anymore 🤡. And the only way to write these circuits is to delve into the weeds and think about this spreadsheet and its regions and whatnot as a "low-level" programmer.

First, let's quickly formalise our public/private inputs:

### Public inputs

- The solution word
- The grid of boxes of 6 words x 5 slots (one for each letter): each cell in the grid is either green, yellow or grey

### Private inputs

- 6 words of 5 letters each

For starters, observe that Wordle's structure is such that every guess is quite independent of the others - if a guess is valid on its own, its always valid inside a game and vice-versa. This signals that one clean structure for the circuit is to make an individual region for each guess.

With this one region per guess construction, let's think about what checks are necessary for each guess:

- The guess must be an English word of 5 letters
- If the grid box at a spot is green, the letter at the corresponding spot of the guess must match the solution's letter
- If a grid box is _not_ green, the letter in the guess at the corresponding spot must _not_ match the solution's letter
- Similar checks follow if the grid box is yellow (and if it is not yellow)

### English word

Typically, in an R1CS circuit, you would make the check for a guess being a dictionary word a Merkle proof: You would make a Merkle tree of all the words in the dictionary and witness the Merkle path of your guess in the tree[^3]. In PLONK/Halo 2 however, you have the added unlock of lookup tables! While it's not particularly efficient to use lookup tables this way (since your circuit will now have 12000+ rows), it is a cool way to make use of the feature, and I wanted to get more familiar with the API so I decided to try this out.

[^3]: Alternately, [you can tightly pack polynomial hashes of words in field elements 🥲](https://github.com/nalinbhardwaj/wordlines)

### Green

Precisely, this check is: for each slot of the grid, if the slot is green, compare the letters at that slot in the guess and the solution. They should be equal.

### Not green

This check is: for each slot of the grid, if the slot is _not_ green, compare the letters at that slot in the guess and the solution. They should _not_ be equal. In other words, the difference of the letters at that slot should be non-zero. We'll use this reformulation later.

### Yellow and not yellow

The check for yellow color works almost the same way: Instead of comparing the letters at the exact slot, the comparison is just replaced by a giant OR on all possible pairings of the guess letter with letters of the solution.

Let's ignore the yellow color boxes for now and just try to lay the intermediate variables out in one region of the spreadsheet, considering only the green boxes:

![image](https://user-images.githubusercontent.com/6984346/178804579-436cf1ca-c4c3-488f-8743-95ad4cd93473.png)

Consider the witness trace of this circuit: We start with the guess (the first row) and the final solution (the second row) and go through a few intermediate computations to obtain the expected value of green boxes. `diff_green`, the third row, is the difference between the letters at the corresponding slots of the solution and the guess (for instance, slot 2, "U" - "L" = 9). Next, to do the two aforementioned green checks, we need to additionally know "is `diff_green` zero?". This'll live in the next row, and finally, we'll have a copy of the output green grid boxes from the instance in the last row to compare with our expected value. Finally, we'll add another column to our spreadsheet that'll just contain the hash of the letters of the guess. This is so we can check the lookup table for the guesses' validation with a single lookup.

Now that we have a high-level intuition for what our circuit should "do", let's figure out how to actually _code_ this and constrain the circuit with Halo 2.

# API

Fundamentally, the Halo 2 API is a way to write functions (or formulas if you will) on the abstraction of the underlying spreadsheet data structure. Since we can't populate each cell and hand wire each constraint by hand, we need a programmatic layer to do this for us.

The Halo 2 library splits this circuit programming into a 2 pass structure: in the first pass, you decide on and assign all the constraints and logical gates that each region/row/cell must abide by. The second pass is assignment/witness generation: you populate values into this spreadsheet and "instantiate" it.[^4]

While I've already mentioned some details of the idea of regions before, a lot of the Halo 2 library is wrapping these regions into tight APIs that reduce programmer overhead. Besides regions that act as locality constructions in the spreadsheet, another accompanying concept introduced by the Halo 2 API is that of **rotation**. Imagine that you are *processing* the spreadsheet row by row, top to bottom. The rotation is just a way to express a row relative to the current row. So the current row is the current "rotation" in this sequential process, the row just above is the "-1" (or "previous") rotation and so forth.

Now, let's pick off our circuit design in the previous section and use the Halo 2 library to codify it:

First, let's make a list of constraints/checks we'll want to add to the spreadsheet:

- The first row (containing the guess) should be an English word (this'll take a lookup check) and we should additionally constrain that the hash of the guess matches the letters in the other columns.
- The second row has no checks
- The third row should check the difference of the corresponding spot on the first two rows matches the difference expressed in this row.
- The fourth row should check that the spots take value 1 only if the row above is zero. This is a rather complicated check - the Halo 2 API allows for a weak abstraction known as a "chip" that allows you to compose smaller sub checks a bit more easily. A chip is really just a fancy word for a "sub" circuit setting up its own gates and witness assignment and being "callable" from a larger circuit.
- Finally, the final row simply needs to check if this spot is 1, the third row must be 0 or if it is 0, the fourth row must be 1.

Now, instantiation and filling in the witness is simply a matter of inputting values according to the mentioned rules.

Ultimately, we have a clean 5 row, 7 column region that asserts everything necessary for one guess. We just repeat 6 of these for each of the possible user guesses to obtain our entire circuit!

Some other miscellaneous notes/thoughts about the Halo 2 API I couldn't fit elsewhere:

- One quirk of the Halo 2 API is that while advice columns are referred to by offset rotations, the instance columns are referred to by absolute row numbers, which adds to some confusion. But this is mostly a function of these instance columns being entirely independent of the regions abstraction.
- Note that the spreadsheet model of layouting is a very intentional choice of the Halo 2 Library. There are many other ways to model ZK circuits while still using them with PLONKish arithmetisation. For instance, Yi Sun/Jonathan Wang from the learning group used only a single column to write their circuit (the [halo2wrong](https://github.com/privacy-scaling-explorations/halo2wrong/blob/master/ecdsa/src/ecdsa.rs) repo does something similar) primarily to reduce verification cost and simplify cost-modelling. On the other hand, Circom developers are planning to stick to the R1CS-like circuit layout structure but just add the ability to define [custom gates](https://github.com/iden3/circom/pull/67) using PLONK. Ultimately, I personally think the generalised many-row many-column spreadsheet-like structure is the most flexible representation amongst these, but there are definitely tradeoffs in ease-of-use vs powerfulness to be explored.
- I love the detail and care put into debug info for the Halo 2 library. Coming from circom-land (where debugging detail is _quite_ lacking to say the least), Halo 2's debugging hand-holding was a breath of fresh air. :)) And I love the little parrot! 🦜

<img width="411" alt="image" src="https://user-images.githubusercontent.com/6984346/178801659-fd532672-e03e-42e6-945f-4c1ac502da1b.png">


[^4]: Sidenote that soundness/under-constraining bugs in this Halo 2 model essentially lie at the margin of the difference of these two passes. This is a useful fact to keep in mind as a circuit writer.


# WASM Port

**The [Halo 2 documentation now has a WASM Guide](https://zcash.github.io/halo2/user/wasm-port.html) based on Zordle.**

Halo 2 is written in Rust and is currently only used by Zcash in their daemon software that runs on metal. As application developers, however, we wanted our circuits to prove and verify in web apps. Pulling together a WASM port of Halo 2 proving and verification was quite non-trivial. Original, my project was a CLI-based Wordle but based on [Uma](https://twitter.com/pumatheuma)'s work on running Halo 2 prover and verifier in-browser, we ported the JS prototype to a React/TS friendly library and further discovered some speedup and memory utilisation tricks to make the Wordle circuit work in browser in a reasonable time frame. These tricks include [Blaine](https://twitter.com/BlaineBublitz)'s discovery of esoteric flags required to [bump up available memory for a Rust ported WebAssembly worker from a random GitHub issue](https://github.com/rustwasm/wasm-bindgen/issues/2498#issuecomment-801494135) and precomputing params and serving them as static files to the Rust WASM. All of these tricks are rolled into a [small test-client](https://github.com/nalinbhardwaj/zordle/tree/main/test-client) that might be helpful to future Halo WASM porters. :)

Feel free to hit me up if you have thoughts on any of the notes in this README, many of these are half-baked thoughts and ideas I'd like to flesh out :))

Thanks to 0xPARC for hosting the learning group and to the 0xPARC community for discussions, reading drafts of this README and everything in between.
