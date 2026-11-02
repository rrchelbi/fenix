import "fmt"

fn main() {
    const a = 7
    const b: i32 = 3

    // The five operators.
    fmt.println(a + b, a - b, a * b, a / b, a % b)

    // Precedence, parentheses and negation.
    var result = (a + b) * -2
    result = result + 1
    fmt.println(result, a + b * 2)
}
