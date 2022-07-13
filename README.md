# Zordle: ZK Wordle

Zordle is [Wordle](https://www.nytimes.com/games/wordle/index.html), but with zero-knowledge proofs. Zordle uses ZK proofs to prove that someone knows words that map to a certain share, but does not reveal those words to a verifier. Zordle is probably the first end-to-end web app built using [Halo 2](https://github.com/zcash/halo2/) ZK proofs!

This project was made as part of [0xPARC Halo 2 Learning Group](https://0xparc.org/blog/halo2-learning-group). Big shoutouts to [Ying Tong](https://twitter.com/therealyingtong) for basically hand-holding me through Halo2 circuit writing and to [Uma](https://twitter.com/pumatheuma) and [Blaine](https://twitter.com/BlaineBublitz) for significant work on porting the Halo 2 library to WASM.

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

# API

# WASM Port



