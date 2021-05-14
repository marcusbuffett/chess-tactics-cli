

<!-- PROJECT LOGO -->
<br />
<p align="center">

  <h3 align="center">Chess Tactics CLI</h3>
</p>



![Screen shot](./assets/usage.gif)

Practice some chess tactics in your terminal while you wait for your code to
compile. Fetches tactics from [this tactics API](https://tactics.exoapi.app).


### Built With

* Rust
* [The Lichess Puzzles Database](https://database.lichess.org/#puzzles)
* [Shakmaty](https://github.com/niklasf/shakmaty)
* [Chess Tactics API](https://tactics.exoapi.app/)

## Installation

```sh
cargo install tactics-trainer-cli
```

<!-- USAGE EXAMPLES -->
## Usage

Usage is straightforward, just run `tactics-trainer`

```sh
tactics-trainer
```
Or specify some tags (See [this
file](https://github.com/ornicar/lila/blob/master/translation/source/puzzleTheme.xml) for all tags):
```sh
tactics-trainer --tags mateIn1
```

Or specify a rating range:
```sh
tactics-trainer --rating=600-1200
```

<!-- ROADMAP -->
## Roadmap

- [ ] Sessions
- [ ] Spaced repetition of failed puzzles
- [ ] AND queries for themes

<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE` for more information.


<!-- CONTACT -->
## Contact

Marcus Bufett - [@marcusbuffett](https://twitter.com/marcusbuffett) - me@mbuffett.com
