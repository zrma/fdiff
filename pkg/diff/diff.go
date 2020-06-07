package diff

import (
	"fmt"
	"log"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
)

func Diff(lhs, rhs string) {
	var err error
	lhs, rhs, err = concat(lhs, rhs)
	if err != nil {
		return
	}

	fmt.Println("diff between ", lhs, "and", rhs)
	fmt.Println("--------------------------------------------")
	lhsList := traverse(lhs)
	rhsList := traverse(rhs)

	diff(lhsList, rhsList)
}

func diff(lhsList, rhsList []string) {
	sort.Strings(lhsList)
	sort.Strings(rhsList)

	var i, j int
	for i < len(lhsList) && j < len(rhsList) {
		switch strings.Compare(lhsList[i], rhsList[j]) {
		case -1:
			fmt.Println(lhsList[i])
			i++
		case 0:
			i++
			j++
			continue
		case 1:
			fmt.Println("                    " + rhsList[j])
			j++
		}
	}
	for i < len(lhsList) {
		fmt.Println(lhsList[i])
		i++
	}
	for j < len(rhsList) {
		fmt.Println("                    " + rhsList[j])
		j++
	}
}

func traverse(route string) []string {
	var res []string
	err := filepath.Walk(route,
		func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if path != route {
				res = append(res,
					strings.TrimPrefix(
						strings.TrimPrefix(path, route),
						"/",
					),
				)
			}
			return nil
		})
	if err != nil {
		log.Println(err)
	}
	return res
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
