import "fmt"

fn main() {
    const a = 7
    const b = -3

    // Comparisons give a bool.
    const bigger = a > b
    fmt.println(bigger, a == b, a != b, a <= 7)

    // `&&` and `||` skip their right side when the left decides.
    var ok: bool = a > 0 && b > 0
    ok = ok || !(a < b)
    fmt.println(ok, b != 0 && a / b < 0)

    if (a > b) {
        fmt.println(a)
    } else {
        fmt.println(b)
    }

    if (b > 0) {
        fmt.println(1)
    } else if (b < 0) {
        fmt.println(-1)
    } else {
        fmt.println(0)
    }

    // An inner block can change an outer variable, and shadow a name.
    var total = 0
    if (a % 2 == 1) {
        const a = 10
        total = total + a
    }
    fmt.println(total, a)
}
