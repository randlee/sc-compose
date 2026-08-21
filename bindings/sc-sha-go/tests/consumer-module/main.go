package main

import (
	"fmt"

	scsha "github.com/randlee/sc-compose/bindings/sc-sha-go/go/sc_sha_go"
)

func main() {
	hash, err := scsha.CalculateHash([]byte("hello\r\n"))
	if err != nil {
		panic(err)
	}
	if hash.Sha256 != "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03" {
		panic(fmt.Sprintf("unexpected normalized hash: %s", hash.Sha256))
	}
	fmt.Println(hash.Sha256)
}
