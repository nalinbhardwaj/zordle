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

Learning about the shiny new features of Halo 2, Wordle seemed like the perfect application to get my hands dirty with them, so I chose to work on this as my project! The rest of this README will be a technical note on the circuits and an informal introduction to the Halo 2 Library and features of PLONKish proving systems.

# Circuit

The over-simplified mental model of Halo 2 I've come to appreciate is that of a giant spreadsheet: You have a cells in a tabular format you can fill values in, mutate them from cell to cell, and check that relationships and constraints you'd desire hold true. Additionally, you have access to some "global" structures that are more powerful than just plain cell relationship comparators: you can check row A is a permutation of row B for a very cheap cost, and its also very cheap to check set membership of the value of a particular cell in a giant list (as long as you can define said giant list at "compile time").

![image](https://user-images.githubusercontent.com/6984346/178658416-78fbd21d-8caf-4d55-97a5-7c0c19e5190c.png)


To be slightly more precise, Halo 2 essentially structures circuits as row-column operations: There are 4 primary types of columns:

- Instance columns: these are best associated with public inputs
- Advice columns: these are best associated with private inputs and the computation trace of the circuit, the "witness" and,
- Fixed columns: constants used in the computation, known at "compile time"
- Selector columns: these are binary values used to "select" particular advice and instance cells and define constraints between them.

Additionally, there's the notion of a lookup column that allows you to check set membership efficiently but that's perhaps best thought of as a giant fixed set instead of a circuit table column.

Of course, the natural question, given this abstraction, is to figure out what's the right way to write efficient ZK circuits inside this playground? Should I use more rows? Or more columns? The answer is quite complicated.

For simpler schemes like Groth16 that are based on R1CS, circuit engineers have universally accepted the metric of "the number of constraints" since most tasks associated with the ZK proof (compilation, proving time, trusted setup compute etc.) scale linearly with the number of non-linear constraints. The structure of PLONK circuits, on the other hand, allows for much more flexibility in defining a circuit, and with it comes a very tough to grasp cost model. There are some rough heuristics however, for instance, more rows make proving time slower (notably, however, the big jump in proving times occurs when number of rows cross powers of 2, where the cost of the intermediate polynomial FFTs required doubles) while more columns make verification time slower. Notably, also, Halo 2, the library, in fact, is very flexible and allows for instantiation of circuits using different polynomial commitment schemes (such as IPA and KZG) which makes cost-modelling instantiation dependent as well.

The abstraction of a spreadsheet for PLONKish arithmetisation is quite powerful because it allows the library to lay out and pack the rows and columns of your circuit tighter automatically (and paves the way for an IR/automated circuit optimiser long term). While great for optimizations, unfortunately this ability to auto-pack comes at the expense of making the API more nuanced and the cost modeling of circuits even more non-trivial to an end circuit programmer.

To elaborate on the _nuance_ of the API, Halo 2 defines the concept of a "region" inside the spreadsheet. A region is the minimal set of cells such that all constraints relating to any single one of them is contained entirely within this region. This is a mouthful, but in essence regions are the minimal building blocks of a circuit. Typically, non-ZK apps are written in disparate modules - a good analogy for this is perhaps the Clock app on your mobile phone: the app is structured coherently to a user (around "time") but if you think about it like a programmer, the timer tab has very little in common with the world clock tab. The same is true for ZK circuits, a Clock circuit might want to check both the world clock and the status of a timer, and the representation of each of these is its own "region" in the Clock circuit. A programmer will write both of them almost independently, and let the compiler figure out the best way to pack them into the spreadsheet.

![image0](https://user-images.githubusercontent.com/6984346/178660515-0d5b74ae-a7e6-4973-b5f3-fb174e305991.jpg)
_Coherency of "time" to a user, but incoherency of regions to a programmer_

While cost-modelling seems to be quite problematic for circuit writers on the surface right now, the general read of participants in the Halo 2 Learning Group seems to be that these APIs give circuit writers enough breathing room to hyper-optimise computation for commonly used primitive circuits, and in future, these efficient primitives can be composed (perhaps inefficiently) into real apps by higher-level circuit writers. Perhaps eventually ZK circuits will get to a point where few low-level programmers will hyper-optimise a minimal instruction set, and the rest of us will just roll our circuits into compilers composing the instruction set.[^2]

[^2]: Supposedly, this will also mark the switching point where we can stop bothering with a hyper-optimised zkEVM and instead just write a zkMIPS for any VM. More notes on this tradeoff [here](https://kelvinfichter.com/pages/thoughts/hybrid-rollups/).

It's certainly fun to theorise about the future, but right now, we have a very real task at hand: making an anti-cheat for a meaningless word game that's not even popular any more. And the only way to write these circuits is delve into the weeds and think deeply about the spreadsheet and its regions and what-not as a "low-level" programmer.

The initial observation I had about Wordle's structure is that every guess is quite independent of the others - if a guess is valid on its own, its always valid inside a game and vice-versa. This made me think that the right structure is to make an individual region for each guess.

With this one region per guess abstraction, let's think about what checks are necessary for each guess:

- It must be a english word of 5 letters
- It must be that if the grid box corresponding to a letter in the word is green, the letter at the corresponding spot must match the solution's letter
- If a grid box is _not_ green, the letter at the corresponding spot must _not_ match
- Similar checks if grid box is yellow (and if its not yellow)

### English word

### Matching letter

### Not matching letter


# API

- rotation is local in a global table
- instance cols are global for no strong reason tbh
- region is a locality construction
- 2 pass structure, pass 1 for constraining, pass 2 for witness gen, and in the margin between the two passes lie all the soundness bugs.

# WASM Port



