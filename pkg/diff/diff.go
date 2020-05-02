package diff

import (
	"fmt"
	"os"
	"path"
	"strings"
)

func Diff(lhs, rhs string) {
	lhs = concatPrefix(lhs)
	rhs = concatPrefix(rhs)

	fmt.Println(lhs)
	fmt.Println(rhs)
}

func concatPrefix(s string) string {
	if !strings.HasPrefix(s, "/") {
		wd, err := os.Getwd()
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			return s
		}
		s = path.Join(wd, s)
	}
	return s
}
