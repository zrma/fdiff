package main

import (
	"github.com/alexflint/go-arg"

	"fdiff/pkg/diff"
)

var args struct {
	LHS string `arg:"positional"`
	RHS string `arg:"positional"`
}

func main() {
	arg.MustParse(&args)
	diff.Diff(args.LHS, args.RHS)
}
