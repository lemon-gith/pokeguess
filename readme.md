# Pokeguess

This app started as the Rust Book's [Guessing Game](https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html) exercise. However, I discovered the [PokeAPI](https://pokeapi.co/) and [Rustemon](https://github.com/mlemesle/rustemon) soon after, which led me to design this, much more complicated, guessing game.

Motivated by the idea that I could make it public and host it on GitHub Pages, I've chosen to make it a Client-Side Rendered webapp, that runs entirely within the browser. And motivated by this being a Rust exercise, I decided to try out the [Yew](https://yew.rs/) front-end framework, too.

## URL

>[!CAUTION]
> The linked URL `pokeguess.piercew.xyz/` is currently dead, because I have not set up the GitHub Actions to build the webapp, nor configured my DNS to the output of that build.

## Notes

```rs
// OMG, USE RUSTEMON TO MAKE A POKEMON GUESSING GAME
// Start off with their description or sprite shadow and have them guess the pokemon
// if they get it wrong, give 3 more clues
// Have different modes, shadow is easy, description is medium, 3 of the other stats is hard :p

/* All modes are 3 strikes and you're out
 * Baby: Sprite -x-> _ -x-> _ -x-> out
 * Easy: Shadow -x-> Desc -x-> Sprite -x-> out
 * Medium: Desc. -x-> _ -x-> Shadow -x-> out
 * Hard: 3 Data -x-> Desc -x-> Shadow -x-> out
 * Extreme: idk yet :}
 */

// TODO: Pick generations before starting game

// TODO: make this using some sort of web-compatible GUI (to be pushed to site when done)

// TODO: See if you can play sounds, if so, could also do pokemon cries :0

pub fn guessing_game() {
  println!("\nWelcome, #TODO: etc. etc.");
  print!("Please pick a game style: ");
  // TODO: keeping it here, because thinking of using GUI
  use std::io::{Write, stdout};
  let _ = Write::flush(&mut stdout());

  let mut input = String::new();
  std::io::stdin().read_line(&mut input).expect("Umm, no?");

  let game_chosen: u8 = input.trim().parse().expect("oi");
  // game_of_guess::guessing_game(arg);
  // TODO: have user pick (game_chosen: u8) in here
  println!("You chose game {game_chosen}");
}
```