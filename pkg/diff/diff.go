package diff

import (
	"fmt"
	"os"
	"path"
	"strings"
)

func Diff(lhs, rhs string) {
	var err error
	lhs, rhs, err = concat(lhs, rhs)
	if err != nil {
		return
	}

	fmt.Println(lhs)
	fmt.Println(rhs)
}

func concat(lhs, rhs string) (string, string, error) {
	var err error
	lhs, err = concatPrefix(lhs)
	if err != nil {
		fmt.Println(err)
		return lhs, rhs, err
	}
	rhs, err = concatPrefix(rhs)
	if err != nil {
		fmt.Println(err)
		return lhs, rhs, err
	}
	return lhs, rhs, nil
}

func concatPrefix(s string) (string, error) {
	if !strings.HasPrefix(s, "/") {
		wd, err := os.Getwd()
		if err != nil {
			return s, err
		}
		s = path.Join(wd, s)

		if _, err := os.Stat(s); os.IsNotExist(err) {
			return s, err
		}
	}
	return s, nil
}
