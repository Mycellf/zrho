# zρ (zee-rho, aka zero)

A programming language for video game hacking. 
Inspired by the EXA VM from Exapunks. 

## zρ programs
* One instruction per line
* Each instruction is 3 characters long
* Semicolons start comments

Acceptable file extensions: .zρ, .zrho

```zρ
; Store the numbers from 0 to 10, inclusive
SET I 0
LBL LOOP
SET D I
ADD I 1 I
JMP I < 10 LOOP
```

Each instruction can have the following types of arguments: 

* Register (r): Stores numbers from -9999 to 9999 inclusive (number of digits depend on the machine and register)
	* General purpose: X, Y, Z, W, etc (amount of these depends on the machine, most only have X and Y)
	* Indexing: I, J, K, L, M (amount depends on the machine, most have only I)
	* Seeking: none planned for now (writing to it adds the value in stead of copying it over, used to index very large arrays) 
	* Storage: D, E, F, G, H (same amount as indexing)
		* The value of the respective indexing register determines the index of the value accessed, out of an array with a set number of elements
		* Most D have 100 elements, although most others have 10
		* G is often big, and H is often massive
		* E and F are often very fast but either store few numbers, or have a more limited range of digits they can store

	* Attempting to read or write an out of bounds value crashes the program. 

* Number (n): A constant provided in the program, within the same range as a register

* Condition (c): A value or comparison expression evaluated as true (≠ 0) or false (= 0)
	* Comparison operations: (>, <, =, >= → ≥, <= → ≤, != → /= → ≠) (a → b → c means a and b auto correct to c)
	* Valid forms are (r/n) and (r/n comparison r/n)

* Label (l): A string provided in the program for use as an identifier

Each register and operation has "time scores" for reading, writing, and evaluation. 
By convention, the G register if present is very slow to write to, and E and F are 
smaller but faster than D. General purpose and indexing registers are most often be
faster than storage registers. 

## Instructions 
Not all machines have every instruction or the same score

(? means optional)

| Operation     | Arguments   | Time Score* | Description                                                                                                                     |
|---------------|-------------|-------------|---------------------------------------------------------------------------------------------------------------------------------|
| `SET`         | `r r/n`     | 1           | sets the first argument to the second argument                                                                                  |
| arithmatic:   |             |             |                                                                                                                                 |
| `ADD`         | `r/n r/n r` | 1           | adds the first two arguments and stores them in the third                                                                       |
| `SUB`         | `r/n r/n r` | 1           | subtract                                                                                                                        |
| `NEG`         | `r`         | 0           | negates the value in the register                                                                                               |
| `MUL`         | `r/n r/n r` | 2           | multiply                                                                                                                        |
| `DIV`         | `r/n r/n r` | 4 or 1      | euclidean division                                                                                                              |
| `REM`         | `r/n r/n r` | 4 or 1      | euclidean remainder aka modulus (this and the above cost only one if they are run just after the other with the same arguments) |
| `ODD`         | `r`         | 0           | sets its argument to 1 if it is odd, else 0, aka takes the argument's value mod 2                                               |
| comparison:   |             |             |                                                                                                                                 |
| `CMP`         | `c r`       | 1           | evaluates the comparison and stores 1 to r if it is true, 0 if false                                                            |
| `TCP`         | `c r`       | 2           | stores 1 to r if the comparison is true                                                                                         |
| `FCP`         | `c r`       | 2           | stores 0 to r if the comparison is false                                                                                        |
| control flow: |             |             |                                                                                                                                 |
| `LBL`         | `l`         | N/A         | marks the next line as a target for a JMP instruction                                                                           |
| `JMP`         | `?c l`      | 1 or 0      | moves program execution to the given label if the first argument is true or not present (has a score of 0 in the latter case)   |
| `LJP`         | `c l`       | 0 or 5      | same as JMP but has a score of 1 if c is true and 5 if c is false, aka likely jump                                              |
| `UJP`         | `c l`       | 0 or 5      | same as JMP but has a score of 5 if c is true and 1 if c is false, aka unlikely jump                                            |
| misc/no-op:   |             |             |                                                                                                                                 |
| `SLP`         | `r/n`       | varies      | takes as many ticks as the passed value                                                                                         |
| `TRY`         | `r`         | 0           | takes as much time as reading from the first argument                                                                           |
| `TRW`         | `r`         | 0           | takes as much time as writing to the first argument                                                                             |
| `CLK`         | `r ?n`      | 0           | copies the total runtime of the program to the first argument, digit shifted right by the second argument                       |
| `END`         |             | N/A         | halts program execution                                                                                                         |

*Default, varies between machines

## Common scores for registers

| Register(s)             | Read | Write |
|-------------------------|------|-------|
| `X` `Y` `Z` `W` `U` `V` | 0    | 0     |
| `I` `J` `K` `L` `M`     | 0    | 0     |
| `D`                     | 1    | 1     |
| `E` `F`                 | 0    | 0     |
| `G`                     | 2    | 4     |
| `H`                     | 2    | 4*    |

*Changing M by more than ±1 disables reading/writing for 16 ticks, attempting during this time will block
