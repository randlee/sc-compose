package sc_sha_go_test

import (
	"encoding/hex"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"testing"

	scsha "github.com/randlee/sc-compose/bindings/sc-sha-go/go/sc_sha_go"
)

type vectors struct {
	HashCases        []hashCase        `json:"hash_cases"`
	CompositionCases []compositionCase `json:"composition_cases"`
}

type hashCase struct {
	Name             string  `json:"name"`
	UTF8FileBytesHex string  `json:"utf8_file_bytes_hex"`
	SHA256           *string `json:"sha256"`
	ErrorCode        *string `json:"error_code"`
}

type compositionCase struct {
	Name      string       `json:"name"`
	Manifest  manifestCase `json:"manifest"`
	SHA256    *string      `json:"sha256"`
	ErrorCode *string      `json:"error_code"`
}

type manifestCase struct {
	Schema string     `json:"schema"`
	Nodes  []nodeCase `json:"nodes"`
	Edges  []edgeCase `json:"edges"`
}

type nodeCase struct {
	Source sourceCase `json:"source"`
	SHA256 string     `json:"sha256"`
}

type edgeCase struct {
	Parent     sourceCase `json:"parent"`
	Child      sourceCase `json:"child"`
	Occurrence uint32     `json:"occurrence"`
}

type sourceCase struct {
	Kind  string `json:"kind"`
	Value string `json:"value"`
}

func readVectors(t *testing.T) vectors {
	t.Helper()
	path := filepath.Join("..", "..", "testdata", "conformance-v1.json")
	contents, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read shared conformance vectors: %v", err)
	}
	var value vectors
	if err := json.Unmarshal(contents, &value); err != nil {
		t.Fatalf("parse shared conformance vectors: %v", err)
	}
	return value
}

func source(t *testing.T, value sourceCase) scsha.CanonicalSource {
	t.Helper()
	switch value.Kind {
	case "local_path":
		return scsha.CanonicalSourceLocalPath{Value: value.Value}
	case "url":
		return scsha.CanonicalSourceUrl{Value: value.Value}
	default:
		t.Fatalf("unknown test source kind %q", value.Kind)
		return nil
	}
}

func manifest(t *testing.T, value manifestCase) scsha.ResolvedTemplateManifest {
	t.Helper()
	nodes := make([]scsha.ResolvedTemplateNode, 0, len(value.Nodes))
	for _, node := range value.Nodes {
		nodes = append(nodes, scsha.ResolvedTemplateNode{
			Source: source(t, node.Source),
			Sha256: node.SHA256,
		})
	}
	edges := make([]scsha.ResolvedIncludeEdge, 0, len(value.Edges))
	for _, edge := range value.Edges {
		edges = append(edges, scsha.ResolvedIncludeEdge{
			Parent:     source(t, edge.Parent),
			Child:      source(t, edge.Child),
			Occurrence: edge.Occurrence,
		})
	}
	return scsha.ResolvedTemplateManifest{Schema: value.Schema, Nodes: nodes, Edges: edges}
}

func stableErrorCode(t *testing.T, err error) string {
	t.Helper()
	if err == nil {
		t.Fatal("expected a typed sc-sha error")
	}
	var invalidUTF8 *scsha.ScShaErrorInvalidUtf8
	if errors.As(err, &invalidUTF8) {
		return invalidUTF8.Code
	}
	var invalidDigest *scsha.ScShaErrorInvalidDigest
	if errors.As(err, &invalidDigest) {
		return invalidDigest.Code
	}
	var invalidSource *scsha.ScShaErrorInvalidCanonicalSource
	if errors.As(err, &invalidSource) {
		return invalidSource.Code
	}
	var invalidManifest *scsha.ScShaErrorInvalidManifest
	if errors.As(err, &invalidManifest) {
		return invalidManifest.Code
	}
	var unsupportedSchema *scsha.ScShaErrorUnsupportedManifestSchema
	if errors.As(err, &unsupportedSchema) {
		return unsupportedSchema.Code
	}
	var duplicateSource *scsha.ScShaErrorDuplicateSource
	if errors.As(err, &duplicateSource) {
		return duplicateSource.Code
	}
	var unknownEndpoint *scsha.ScShaErrorUnknownEdgeEndpoint
	if errors.As(err, &unknownEndpoint) {
		return unknownEndpoint.Code
	}
	t.Fatalf("unexpected error type %T: %v", err, err)
	return ""
}

func TestCalculateHashMatchesSharedVectors(t *testing.T) {
	for _, testCase := range readVectors(t).HashCases {
		t.Run(testCase.Name, func(t *testing.T) {
			input, err := hex.DecodeString(testCase.UTF8FileBytesHex)
			if err != nil {
				t.Fatalf("decode test bytes: %v", err)
			}
			actual, err := scsha.CalculateHash(input)
			if testCase.SHA256 != nil {
				if err != nil {
					t.Fatalf("calculate hash: %v", err)
				}
				if actual.Sha256 != *testCase.SHA256 {
					t.Fatalf("digest = %s, want %s", actual.Sha256, *testCase.SHA256)
				}
				return
			}
			if testCase.ErrorCode == nil {
				t.Fatal("vector must define a digest or error code")
			}
			if code := stableErrorCode(t, err); code != *testCase.ErrorCode {
				t.Fatalf("error code = %s, want %s", code, *testCase.ErrorCode)
			}
		})
	}
}

func TestCalculateCompositionHashMatchesSharedVectors(t *testing.T) {
	for _, testCase := range readVectors(t).CompositionCases {
		t.Run(testCase.Name, func(t *testing.T) {
			actual, err := scsha.CalculateCompositionHash(manifest(t, testCase.Manifest))
			if testCase.SHA256 != nil {
				if err != nil {
					t.Fatalf("calculate composition hash: %v", err)
				}
				if actual.Sha256 != *testCase.SHA256 {
					t.Fatalf("digest = %s, want %s", actual.Sha256, *testCase.SHA256)
				}
				return
			}
			if testCase.ErrorCode == nil {
				t.Fatal("vector must define a digest or error code")
			}
			if code := stableErrorCode(t, err); code != *testCase.ErrorCode {
				t.Fatalf("error code = %s, want %s", code, *testCase.ErrorCode)
			}
		})
	}
}
