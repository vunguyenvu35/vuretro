# Macros and moves

A **macro** makes one key (or one gamepad button) do more than one thing. This guide shows how to
make a **Move**: a special move or a whole combo that a fighting game needs a long input for,
played with one key. It also covers profiles (one set of moves per character), diagonal keys and
Record.

Macros are loaded **when a game starts**. After you add or change one, close the game and open it
again.

## 1. Make a move in five steps

1. Open **Macros** in the sidebar (or, in the library, select the game and press **Macros for this
   game…**).
2. Check **Macros for**. There are three levels, played together while you play:
   **All Neo Geo games** (every game of that system), **one game** (and its character profiles),
   and **Every game**. Pick *All Neo Geo games* for moves shared by every fighter on the system, or
   one game for that game's own. Moves are written with Neo Geo buttons, so **+ Move** only
   appears for Neo Geo (the system, or a Neo Geo game); other systems get Combo, Turbo and Sequence.
3. Press **+ Move**. A card appears and waits for its **trigger**: press the key you want to play the
   move with, for example `V`. (Esc cancels; **+ Gamepad button** uses a pad button instead.)
4. In **Move**, type the move, for example `236P`, or press one of the buttons under **Start from**
   (`236P`, `623P`, `623623AB`, `j.D`…) to fill it in; press more to chain them with commas, and
   **Clear** to start over. The card tells you at once how many steps it makes
   (`✓ 3 steps · 0.15 s`), or why it cannot play it.
5. Close the game if it is running, start it again, and press `V` in the game.

Choose a trigger key that is **not** one of the game's own buttons (by default the arrows and `Z X A S
Q E W R`), because a game button still sends its normal button as well. `Esc`, `Tab`, `Backspace`,
`P` and `F1`–`F12` are used by the game window and cannot be triggers.

## 2. Writing a move

Directions are numpad digits, **as seen when the character faces right**:

| | | |
|---|---|---|
| `7` ↖ | `8` ↑ | `9` ↗ |
| `4` ← | `5` neutral | `6` → |
| `1` ↙ | `2` ↓ | `3` ↘ |

Buttons are the four Neo Geo buttons `A B C D`. `P` stands for a punch (it plays `A`) and `K` for a
kick (`B`). Write `A+B`, `A + B` or `AB` for two buttons at once.

| Write | Means |
|---|---|
| `236P` | quarter circle forward, then a punch (the buttons go down with the last direction) |
| `623P`, `DPP` | dragon punch motion, `DP` is `623` |
| `QCF` `QCB` `DP` `RDP` `HCF` `HCB` `DQCF` | named motions: `236 214 623 421 41236 63214 236236` |
| `623623AB` | double dragon punch with A+B at once |
| `[4]6A` | charge: hold back, then forward with A |
| `cr.B` | crouching B (down is held with the button) |
| `j.D` | jumping D (up, a short wait in the air, then D) |
| `cl.C`, `far.C` | standing close or far C (plays plain `C`) |
| `,` | **link**: the next move starts when the first has finished |
| `>` | **cancel**: the next move starts at once |
| `cr.A x3` | repeat three times |
| `6A/6B` | alternatives: the first one is played |
| `( … )` | optional: left out |

Letters are not case sensitive. A combo is moves joined with `,` and `>`:

```
j.D, cl.C > 236P        jump-in, close heavy punch, cancelled into a fireball
cr.B, cr.B, cr.A > 623P three light normals, then a special
```

Words that are not input, such as `special` or `DM`, cannot be played: the card shows the part it
does not understand. Replace them with the real input of the character.

### Which way it faces

Directions depend on the side the character is on, so a move has a **Facing**:

- **Auto** (the default for a new move): when you press the trigger, **hold ← or → towards your
  opponent**. Holding ← plays the move mirrored, holding → (or nothing) plays it as written. One move
  works on both sides.
- **Right →** or **← Left**: always as written, or always mirrored.

### Buttons are named like on a Neo Geo

On a Neo Geo game the buttons are called `A B C D`: `Z X A S` on the default keys. A standard pad
calls the same four `B A Y X`; the **Buttons: Neo Geo | Standard pad** switch next to **Macros for**
changes the names (it follows the game by default). The game receives the same buttons either way.

## 3. Playing diagonals on a keyboard

Pressing two arrows together plays a diagonal. If that is awkward, or your keyboard has no numpad
(a TKL keyboard), open **Controls** and, in **Diagonal keys**, give ↖ ↗ ↙ ↘ a key each. One key then
presses both directions. They are keyboard only (a pad has its D-pad), and **Record** understands them.

## 4. One set of moves per character: profiles

Every character has different moves. Under **Macros for** there is a **Profile** row:

- **All characters**: macros the whole game shares.
- **+ New profile**: a set for one character. Give it a name (for example `Haohmaru`), add its moves,
  and press **Use this profile**.
- Only **one profile is in use** at a time, shown as *in use*. While you play, the macros of the
  system, **All characters**, the profile in use and the macros for every game play together.
- Change the profile before you play from the game's details in the library: **Macros ▸ Profile in
  use**. **Macros for this game…** there opens this screen for that game.

## 5. Other ways to make a macro

- **Record** (a **Sequence**): press **+ Sequence**, then **● Record** and play the moves on the keyboard
  with the keys you set in Controls. **Stop** ends it (so does `Esc`). Every press becomes a step, and
  the time between presses becomes a **wait** row. Press **Shorten waits** to make each wait at most
  100 ms, or a recording played at human speed will be slow. Edit any step by hand.
- **Edit steps by hand**: on a Move, turns it into a sequence you can change step by step.
- **Combo** (hold the key to press several buttons together) and **Turbo** (hold the key to tap the
  buttons again and again, 2–30 times a second) are simple kinds for everyday use.

## 6. If a move does not come out

- The macros load at the start of a game: close the game and open it again.
- Check the card: the `✓ N steps` line must be there, and the **Trigger** must be set.
- The right character profile must be **in use**.
- The character faces the other way: use **Auto** facing and hold the direction towards the opponent,
  or choose **← Left**.
- The timings are an approximation of 60 frames per second play (3 frames per direction or button,
  about 8 for a link). A game with a strict window may need them changed: use **Edit steps by hand**
  and lengthen the steps.
- Super moves need their gauge full: a macro does not change that.
- Pressing the trigger while a move is playing does nothing; it plays once per press.
