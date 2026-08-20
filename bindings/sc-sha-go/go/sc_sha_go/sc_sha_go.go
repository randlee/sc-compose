package sc_sha_go

// #include <sc_sha_go.h>
import "C"

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"io"
	"math"
	"unsafe"
)

// This is needed, because as of go 1.24
// type RustBuffer C.RustBuffer cannot have methods,
// RustBuffer is treated as non-local type
type GoRustBuffer struct {
	inner C.RustBuffer
}

type RustBufferI interface {
	AsReader() *bytes.Reader
	Free()
	ToGoBytes() []byte
	Data() unsafe.Pointer
	Len() uint64
	Capacity() uint64
}

// C.RustBuffer fields exposed as an interface so they can be accessed in different Go packages.
// See https://github.com/golang/go/issues/13467
type ExternalCRustBuffer interface {
	Data() unsafe.Pointer
	Len() uint64
	Capacity() uint64
}

func RustBufferFromC(b C.RustBuffer) ExternalCRustBuffer {
	return GoRustBuffer{
		inner: b,
	}
}

func CFromRustBuffer(b ExternalCRustBuffer) C.RustBuffer {
	return C.RustBuffer{
		capacity: C.uint64_t(b.Capacity()),
		len:      C.uint64_t(b.Len()),
		data:     (*C.uchar)(b.Data()),
	}
}

func RustBufferFromExternal(b ExternalCRustBuffer) GoRustBuffer {
	return GoRustBuffer{
		inner: C.RustBuffer{
			capacity: C.uint64_t(b.Capacity()),
			len:      C.uint64_t(b.Len()),
			data:     (*C.uchar)(b.Data()),
		},
	}
}

func (cb GoRustBuffer) Capacity() uint64 {
	return uint64(cb.inner.capacity)
}

func (cb GoRustBuffer) Len() uint64 {
	return uint64(cb.inner.len)
}

func (cb GoRustBuffer) Data() unsafe.Pointer {
	return unsafe.Pointer(cb.inner.data)
}

func (cb GoRustBuffer) AsReader() *bytes.Reader {
	b := unsafe.Slice((*byte)(cb.inner.data), C.uint64_t(cb.inner.len))
	return bytes.NewReader(b)
}

func (cb GoRustBuffer) Free() {
	rustCall(func(status *C.RustCallStatus) bool {
		C.ffi_sc_sha_go_rustbuffer_free(cb.inner, status)
		return false
	})
}

func (cb GoRustBuffer) ToGoBytes() []byte {
	return C.GoBytes(unsafe.Pointer(cb.inner.data), C.int(cb.inner.len))
}

func stringToRustBuffer(str string) C.RustBuffer {
	return bytesToRustBuffer([]byte(str))
}

func bytesToRustBuffer(b []byte) C.RustBuffer {
	if len(b) == 0 {
		return C.RustBuffer{}
	}
	// We can pass the pointer along here, as it is pinned
	// for the duration of this call
	foreign := C.ForeignBytes{
		len:  C.int(len(b)),
		data: (*C.uchar)(unsafe.Pointer(&b[0])),
	}

	return rustCall(func(status *C.RustCallStatus) C.RustBuffer {
		return C.ffi_sc_sha_go_rustbuffer_from_bytes(foreign, status)
	})
}

type BufLifter[GoType any] interface {
	Lift(value RustBufferI) GoType
}

type BufLowerer[GoType any] interface {
	Lower(value GoType) C.RustBuffer
}

type BufReader[GoType any] interface {
	Read(reader io.Reader) GoType
}

type BufWriter[GoType any] interface {
	Write(writer io.Writer, value GoType)
}

func LowerIntoRustBuffer[GoType any](bufWriter BufWriter[GoType], value GoType) C.RustBuffer {
	// This might be not the most efficient way but it does not require knowing allocation size
	// beforehand
	var buffer bytes.Buffer
	bufWriter.Write(&buffer, value)

	bytes, err := io.ReadAll(&buffer)
	if err != nil {
		panic(fmt.Errorf("reading written data: %w", err))
	}
	return bytesToRustBuffer(bytes)
}

func LiftFromRustBuffer[GoType any](bufReader BufReader[GoType], rbuf RustBufferI) GoType {
	defer rbuf.Free()
	reader := rbuf.AsReader()
	item := bufReader.Read(reader)
	if reader.Len() > 0 {
		// TODO: Remove this
		leftover, _ := io.ReadAll(reader)
		panic(fmt.Errorf("Junk remaining in buffer after lifting: %s", string(leftover)))
	}
	return item
}

func rustCallWithError[E any, U any](converter BufReader[E], callback func(*C.RustCallStatus) U) (U, E) {
	var status C.RustCallStatus
	returnValue := callback(&status)
	err := checkCallStatus(converter, status)
	return returnValue, err
}

func checkCallStatus[E any](converter BufReader[E], status C.RustCallStatus) E {
	switch status.code {
	case 0:
		var zero E
		return zero
	case 1:
		return LiftFromRustBuffer(converter, GoRustBuffer{inner: status.errorBuf})
	case 2:
		// when the rust code sees a panic, it tries to construct a rustBuffer
		// with the message.  but if that code panics, then it just sends back
		// an empty buffer.
		if status.errorBuf.len > 0 {
			panic(fmt.Errorf("%s", FfiConverterStringINSTANCE.Lift(GoRustBuffer{inner: status.errorBuf})))
		} else {
			panic(fmt.Errorf("Rust panicked while handling Rust panic"))
		}
	default:
		panic(fmt.Errorf("unknown status code: %d", status.code))
	}
}

func checkCallStatusUnknown(status C.RustCallStatus) error {
	switch status.code {
	case 0:
		return nil
	case 1:
		panic(fmt.Errorf("function not returning an error returned an error"))
	case 2:
		// when the rust code sees a panic, it tries to construct a C.RustBuffer
		// with the message.  but if that code panics, then it just sends back
		// an empty buffer.
		if status.errorBuf.len > 0 {
			panic(fmt.Errorf("%s", FfiConverterStringINSTANCE.Lift(GoRustBuffer{
				inner: status.errorBuf,
			})))
		} else {
			panic(fmt.Errorf("Rust panicked while handling Rust panic"))
		}
	default:
		return fmt.Errorf("unknown status code: %d", status.code)
	}
}

func rustCall[U any](callback func(*C.RustCallStatus) U) U {
	returnValue, err := rustCallWithError[error](nil, callback)
	if err != nil {
		panic(err)
	}
	return returnValue
}

type NativeError interface {
	AsError() error
}

func writeInt8(writer io.Writer, value int8) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint8(writer io.Writer, value uint8) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt16(writer io.Writer, value int16) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint16(writer io.Writer, value uint16) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt32(writer io.Writer, value int32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint32(writer io.Writer, value uint32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt64(writer io.Writer, value int64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint64(writer io.Writer, value uint64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeFloat32(writer io.Writer, value float32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeFloat64(writer io.Writer, value float64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func readInt8(reader io.Reader) int8 {
	var result int8
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint8(reader io.Reader) uint8 {
	var result uint8
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt16(reader io.Reader) int16 {
	var result int16
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint16(reader io.Reader) uint16 {
	var result uint16
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt32(reader io.Reader) int32 {
	var result int32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint32(reader io.Reader) uint32 {
	var result uint32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt64(reader io.Reader) int64 {
	var result int64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint64(reader io.Reader) uint64 {
	var result uint64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readFloat32(reader io.Reader) float32 {
	var result float32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readFloat64(reader io.Reader) float64 {
	var result float64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func init() {

	uniffiCheckChecksums()
}

func uniffiCheckChecksums() {
	// Get the bindings contract version from our ComponentInterface
	bindingsContractVersion := 30
	// Get the scaffolding contract version by calling the into the dylib
	scaffoldingContractVersion := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint32_t {
		return C.ffi_sc_sha_go_uniffi_contract_version()
	})
	if bindingsContractVersion != int(scaffoldingContractVersion) {
		// If this happens try cleaning and rebuilding your project
		panic("sc_sha_go: UniFFI contract version mismatch")
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_sc_sha_go_checksum_func_calculate_composition_hash()
		})
		if checksum != 45417 {
			// If this happens try cleaning and rebuilding your project
			panic("sc_sha_go: uniffi_sc_sha_go_checksum_func_calculate_composition_hash: UniFFI API checksum mismatch")
		}
	}
	{
		checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
			return C.uniffi_sc_sha_go_checksum_func_calculate_hash()
		})
		if checksum != 50462 {
			// If this happens try cleaning and rebuilding your project
			panic("sc_sha_go: uniffi_sc_sha_go_checksum_func_calculate_hash: UniFFI API checksum mismatch")
		}
	}
}

type FfiConverterUint32 struct{}

var FfiConverterUint32INSTANCE = FfiConverterUint32{}

func (FfiConverterUint32) Lower(value uint32) C.uint32_t {
	return C.uint32_t(value)
}

func (FfiConverterUint32) Write(writer io.Writer, value uint32) {
	writeUint32(writer, value)
}

func (FfiConverterUint32) Lift(value C.uint32_t) uint32 {
	return uint32(value)
}

func (FfiConverterUint32) Read(reader io.Reader) uint32 {
	return readUint32(reader)
}

type FfiDestroyerUint32 struct{}

func (FfiDestroyerUint32) Destroy(_ uint32) {}

type FfiConverterString struct{}

var FfiConverterStringINSTANCE = FfiConverterString{}

func (FfiConverterString) Lift(rb RustBufferI) string {
	defer rb.Free()
	reader := rb.AsReader()
	b, err := io.ReadAll(reader)
	if err != nil {
		panic(fmt.Errorf("reading reader: %w", err))
	}
	return string(b)
}

func (FfiConverterString) Read(reader io.Reader) string {
	length := readInt32(reader)
	buffer := make([]byte, length)
	read_length, err := reader.Read(buffer)
	if err != nil && err != io.EOF {
		panic(err)
	}
	if read_length != int(length) {
		panic(fmt.Errorf("bad read length when reading string, expected %d, read %d", length, read_length))
	}
	return string(buffer)
}

func (FfiConverterString) Lower(value string) C.RustBuffer {
	return stringToRustBuffer(value)
}

func (c FfiConverterString) LowerExternal(value string) ExternalCRustBuffer {
	return RustBufferFromC(stringToRustBuffer(value))
}

func (FfiConverterString) Write(writer io.Writer, value string) {
	if len(value) > math.MaxInt32 {
		panic("String is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	write_length, err := io.WriteString(writer, value)
	if err != nil {
		panic(err)
	}
	if write_length != len(value) {
		panic(fmt.Errorf("bad write length when writing string, expected %d, written %d", len(value), write_length))
	}
}

type FfiDestroyerString struct{}

func (FfiDestroyerString) Destroy(_ string) {}

type FfiConverterBytes struct{}

var FfiConverterBytesINSTANCE = FfiConverterBytes{}

func (c FfiConverterBytes) Lower(value []byte) C.RustBuffer {
	return LowerIntoRustBuffer[[]byte](c, value)
}

func (c FfiConverterBytes) LowerExternal(value []byte) ExternalCRustBuffer {
	return RustBufferFromC(c.Lower(value))
}

func (c FfiConverterBytes) Write(writer io.Writer, value []byte) {
	if len(value) > math.MaxInt32 {
		panic("[]byte is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	write_length, err := writer.Write(value)
	if err != nil {
		panic(err)
	}
	if write_length != len(value) {
		panic(fmt.Errorf("bad write length when writing []byte, expected %d, written %d", len(value), write_length))
	}
}

func (c FfiConverterBytes) Lift(rb RustBufferI) []byte {
	return LiftFromRustBuffer[[]byte](c, rb)
}

func (c FfiConverterBytes) Read(reader io.Reader) []byte {
	length := readInt32(reader)
	buffer := make([]byte, length)
	read_length, err := reader.Read(buffer)
	if err != nil && err != io.EOF {
		panic(err)
	}
	if read_length != int(length) {
		panic(fmt.Errorf("bad read length when reading []byte, expected %d, read %d", length, read_length))
	}
	return buffer
}

type FfiDestroyerBytes struct{}

func (FfiDestroyerBytes) Destroy(_ []byte) {}

type CompositionHash struct {
	Sha256 string
}

func (r *CompositionHash) Destroy() {
	FfiDestroyerString{}.Destroy(r.Sha256)
}

type FfiConverterCompositionHash struct{}

var FfiConverterCompositionHashINSTANCE = FfiConverterCompositionHash{}

func (c FfiConverterCompositionHash) Lift(rb RustBufferI) CompositionHash {
	return LiftFromRustBuffer[CompositionHash](c, rb)
}

func (c FfiConverterCompositionHash) Read(reader io.Reader) CompositionHash {
	return CompositionHash{
		FfiConverterStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterCompositionHash) Lower(value CompositionHash) C.RustBuffer {
	return LowerIntoRustBuffer[CompositionHash](c, value)
}

func (c FfiConverterCompositionHash) LowerExternal(value CompositionHash) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[CompositionHash](c, value))
}

func (c FfiConverterCompositionHash) Write(writer io.Writer, value CompositionHash) {
	FfiConverterStringINSTANCE.Write(writer, value.Sha256)
}

type FfiDestroyerCompositionHash struct{}

func (_ FfiDestroyerCompositionHash) Destroy(value CompositionHash) {
	value.Destroy()
}

type ResolvedIncludeEdge struct {
	Parent     CanonicalSource
	Child      CanonicalSource
	Occurrence uint32
}

func (r *ResolvedIncludeEdge) Destroy() {
	FfiDestroyerCanonicalSource{}.Destroy(r.Parent)
	FfiDestroyerCanonicalSource{}.Destroy(r.Child)
	FfiDestroyerUint32{}.Destroy(r.Occurrence)
}

type FfiConverterResolvedIncludeEdge struct{}

var FfiConverterResolvedIncludeEdgeINSTANCE = FfiConverterResolvedIncludeEdge{}

func (c FfiConverterResolvedIncludeEdge) Lift(rb RustBufferI) ResolvedIncludeEdge {
	return LiftFromRustBuffer[ResolvedIncludeEdge](c, rb)
}

func (c FfiConverterResolvedIncludeEdge) Read(reader io.Reader) ResolvedIncludeEdge {
	return ResolvedIncludeEdge{
		FfiConverterCanonicalSourceINSTANCE.Read(reader),
		FfiConverterCanonicalSourceINSTANCE.Read(reader),
		FfiConverterUint32INSTANCE.Read(reader),
	}
}

func (c FfiConverterResolvedIncludeEdge) Lower(value ResolvedIncludeEdge) C.RustBuffer {
	return LowerIntoRustBuffer[ResolvedIncludeEdge](c, value)
}

func (c FfiConverterResolvedIncludeEdge) LowerExternal(value ResolvedIncludeEdge) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[ResolvedIncludeEdge](c, value))
}

func (c FfiConverterResolvedIncludeEdge) Write(writer io.Writer, value ResolvedIncludeEdge) {
	FfiConverterCanonicalSourceINSTANCE.Write(writer, value.Parent)
	FfiConverterCanonicalSourceINSTANCE.Write(writer, value.Child)
	FfiConverterUint32INSTANCE.Write(writer, value.Occurrence)
}

type FfiDestroyerResolvedIncludeEdge struct{}

func (_ FfiDestroyerResolvedIncludeEdge) Destroy(value ResolvedIncludeEdge) {
	value.Destroy()
}

type ResolvedTemplateManifest struct {
	Schema string
	Nodes  []ResolvedTemplateNode
	Edges  []ResolvedIncludeEdge
}

func (r *ResolvedTemplateManifest) Destroy() {
	FfiDestroyerString{}.Destroy(r.Schema)
	FfiDestroyerSequenceResolvedTemplateNode{}.Destroy(r.Nodes)
	FfiDestroyerSequenceResolvedIncludeEdge{}.Destroy(r.Edges)
}

type FfiConverterResolvedTemplateManifest struct{}

var FfiConverterResolvedTemplateManifestINSTANCE = FfiConverterResolvedTemplateManifest{}

func (c FfiConverterResolvedTemplateManifest) Lift(rb RustBufferI) ResolvedTemplateManifest {
	return LiftFromRustBuffer[ResolvedTemplateManifest](c, rb)
}

func (c FfiConverterResolvedTemplateManifest) Read(reader io.Reader) ResolvedTemplateManifest {
	return ResolvedTemplateManifest{
		FfiConverterStringINSTANCE.Read(reader),
		FfiConverterSequenceResolvedTemplateNodeINSTANCE.Read(reader),
		FfiConverterSequenceResolvedIncludeEdgeINSTANCE.Read(reader),
	}
}

func (c FfiConverterResolvedTemplateManifest) Lower(value ResolvedTemplateManifest) C.RustBuffer {
	return LowerIntoRustBuffer[ResolvedTemplateManifest](c, value)
}

func (c FfiConverterResolvedTemplateManifest) LowerExternal(value ResolvedTemplateManifest) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[ResolvedTemplateManifest](c, value))
}

func (c FfiConverterResolvedTemplateManifest) Write(writer io.Writer, value ResolvedTemplateManifest) {
	FfiConverterStringINSTANCE.Write(writer, value.Schema)
	FfiConverterSequenceResolvedTemplateNodeINSTANCE.Write(writer, value.Nodes)
	FfiConverterSequenceResolvedIncludeEdgeINSTANCE.Write(writer, value.Edges)
}

type FfiDestroyerResolvedTemplateManifest struct{}

func (_ FfiDestroyerResolvedTemplateManifest) Destroy(value ResolvedTemplateManifest) {
	value.Destroy()
}

type ResolvedTemplateNode struct {
	Source CanonicalSource
	Sha256 string
}

func (r *ResolvedTemplateNode) Destroy() {
	FfiDestroyerCanonicalSource{}.Destroy(r.Source)
	FfiDestroyerString{}.Destroy(r.Sha256)
}

type FfiConverterResolvedTemplateNode struct{}

var FfiConverterResolvedTemplateNodeINSTANCE = FfiConverterResolvedTemplateNode{}

func (c FfiConverterResolvedTemplateNode) Lift(rb RustBufferI) ResolvedTemplateNode {
	return LiftFromRustBuffer[ResolvedTemplateNode](c, rb)
}

func (c FfiConverterResolvedTemplateNode) Read(reader io.Reader) ResolvedTemplateNode {
	return ResolvedTemplateNode{
		FfiConverterCanonicalSourceINSTANCE.Read(reader),
		FfiConverterStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterResolvedTemplateNode) Lower(value ResolvedTemplateNode) C.RustBuffer {
	return LowerIntoRustBuffer[ResolvedTemplateNode](c, value)
}

func (c FfiConverterResolvedTemplateNode) LowerExternal(value ResolvedTemplateNode) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[ResolvedTemplateNode](c, value))
}

func (c FfiConverterResolvedTemplateNode) Write(writer io.Writer, value ResolvedTemplateNode) {
	FfiConverterCanonicalSourceINSTANCE.Write(writer, value.Source)
	FfiConverterStringINSTANCE.Write(writer, value.Sha256)
}

type FfiDestroyerResolvedTemplateNode struct{}

func (_ FfiDestroyerResolvedTemplateNode) Destroy(value ResolvedTemplateNode) {
	value.Destroy()
}

type TemplateHash struct {
	Sha256 string
}

func (r *TemplateHash) Destroy() {
	FfiDestroyerString{}.Destroy(r.Sha256)
}

type FfiConverterTemplateHash struct{}

var FfiConverterTemplateHashINSTANCE = FfiConverterTemplateHash{}

func (c FfiConverterTemplateHash) Lift(rb RustBufferI) TemplateHash {
	return LiftFromRustBuffer[TemplateHash](c, rb)
}

func (c FfiConverterTemplateHash) Read(reader io.Reader) TemplateHash {
	return TemplateHash{
		FfiConverterStringINSTANCE.Read(reader),
	}
}

func (c FfiConverterTemplateHash) Lower(value TemplateHash) C.RustBuffer {
	return LowerIntoRustBuffer[TemplateHash](c, value)
}

func (c FfiConverterTemplateHash) LowerExternal(value TemplateHash) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[TemplateHash](c, value))
}

func (c FfiConverterTemplateHash) Write(writer io.Writer, value TemplateHash) {
	FfiConverterStringINSTANCE.Write(writer, value.Sha256)
}

type FfiDestroyerTemplateHash struct{}

func (_ FfiDestroyerTemplateHash) Destroy(value TemplateHash) {
	value.Destroy()
}

type CanonicalSource interface {
	Destroy()
}
type CanonicalSourceLocalPath struct {
	Value string
}

func (e CanonicalSourceLocalPath) Destroy() {
	FfiDestroyerString{}.Destroy(e.Value)
}

type CanonicalSourceUrl struct {
	Value string
}

func (e CanonicalSourceUrl) Destroy() {
	FfiDestroyerString{}.Destroy(e.Value)
}

type FfiConverterCanonicalSource struct{}

var FfiConverterCanonicalSourceINSTANCE = FfiConverterCanonicalSource{}

func (c FfiConverterCanonicalSource) Lift(rb RustBufferI) CanonicalSource {
	return LiftFromRustBuffer[CanonicalSource](c, rb)
}

func (c FfiConverterCanonicalSource) Lower(value CanonicalSource) C.RustBuffer {
	return LowerIntoRustBuffer[CanonicalSource](c, value)
}

func (c FfiConverterCanonicalSource) LowerExternal(value CanonicalSource) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[CanonicalSource](c, value))
}
func (FfiConverterCanonicalSource) Read(reader io.Reader) CanonicalSource {
	id := readInt32(reader)
	switch id {
	case 1:
		return CanonicalSourceLocalPath{
			FfiConverterStringINSTANCE.Read(reader),
		}
	case 2:
		return CanonicalSourceUrl{
			FfiConverterStringINSTANCE.Read(reader),
		}
	default:
		panic(fmt.Sprintf("invalid enum value %v in FfiConverterCanonicalSource.Read()", id))
	}
}

func (FfiConverterCanonicalSource) Write(writer io.Writer, value CanonicalSource) {
	switch variant_value := value.(type) {
	case CanonicalSourceLocalPath:
		writeInt32(writer, 1)
		FfiConverterStringINSTANCE.Write(writer, variant_value.Value)
	case CanonicalSourceUrl:
		writeInt32(writer, 2)
		FfiConverterStringINSTANCE.Write(writer, variant_value.Value)
	default:
		_ = variant_value
		panic(fmt.Sprintf("invalid enum value `%v` in FfiConverterCanonicalSource.Write", value))
	}
}

type FfiDestroyerCanonicalSource struct{}

func (_ FfiDestroyerCanonicalSource) Destroy(value CanonicalSource) {
	value.Destroy()
}

type ScShaError struct {
	err error
}

// Convenience method to turn *ScShaError into error
// Avoiding treating nil pointer as non nil error interface
func (err *ScShaError) AsError() error {
	if err == nil {
		return nil
	} else {
		return err
	}
}

func (err ScShaError) Error() string {
	return fmt.Sprintf("ScShaError: %s", err.err.Error())
}

func (err ScShaError) Unwrap() error {
	return err.err
}

// Err* are used for checking error type with `errors.Is`
var ErrScShaErrorInvalidUtf8 = fmt.Errorf("ScShaErrorInvalidUtf8")
var ErrScShaErrorInvalidDigest = fmt.Errorf("ScShaErrorInvalidDigest")
var ErrScShaErrorInvalidCanonicalSource = fmt.Errorf("ScShaErrorInvalidCanonicalSource")
var ErrScShaErrorInvalidManifest = fmt.Errorf("ScShaErrorInvalidManifest")
var ErrScShaErrorUnsupportedManifestSchema = fmt.Errorf("ScShaErrorUnsupportedManifestSchema")
var ErrScShaErrorDuplicateSource = fmt.Errorf("ScShaErrorDuplicateSource")
var ErrScShaErrorUnknownEdgeEndpoint = fmt.Errorf("ScShaErrorUnknownEdgeEndpoint")

// Variant structs
type ScShaErrorInvalidUtf8 struct {
	Code    string
	Message string
}

func NewScShaErrorInvalidUtf8(
	code string,
	message string,
) *ScShaError {
	return &ScShaError{err: &ScShaErrorInvalidUtf8{
		Code:    code,
		Message: message}}
}

func (e ScShaErrorInvalidUtf8) destroy() {
	FfiDestroyerString{}.Destroy(e.Code)
	FfiDestroyerString{}.Destroy(e.Message)
}

func (err ScShaErrorInvalidUtf8) Error() string {
	return fmt.Sprint("InvalidUtf8",
		": ",

		"Code=",
		err.Code,
		", ",
		"Message=",
		err.Message,
	)
}

func (self ScShaErrorInvalidUtf8) Is(target error) bool {
	return target == ErrScShaErrorInvalidUtf8
}

type ScShaErrorInvalidDigest struct {
	Code    string
	Message string
}

func NewScShaErrorInvalidDigest(
	code string,
	message string,
) *ScShaError {
	return &ScShaError{err: &ScShaErrorInvalidDigest{
		Code:    code,
		Message: message}}
}

func (e ScShaErrorInvalidDigest) destroy() {
	FfiDestroyerString{}.Destroy(e.Code)
	FfiDestroyerString{}.Destroy(e.Message)
}

func (err ScShaErrorInvalidDigest) Error() string {
	return fmt.Sprint("InvalidDigest",
		": ",

		"Code=",
		err.Code,
		", ",
		"Message=",
		err.Message,
	)
}

func (self ScShaErrorInvalidDigest) Is(target error) bool {
	return target == ErrScShaErrorInvalidDigest
}

type ScShaErrorInvalidCanonicalSource struct {
	Code    string
	Message string
}

func NewScShaErrorInvalidCanonicalSource(
	code string,
	message string,
) *ScShaError {
	return &ScShaError{err: &ScShaErrorInvalidCanonicalSource{
		Code:    code,
		Message: message}}
}

func (e ScShaErrorInvalidCanonicalSource) destroy() {
	FfiDestroyerString{}.Destroy(e.Code)
	FfiDestroyerString{}.Destroy(e.Message)
}

func (err ScShaErrorInvalidCanonicalSource) Error() string {
	return fmt.Sprint("InvalidCanonicalSource",
		": ",

		"Code=",
		err.Code,
		", ",
		"Message=",
		err.Message,
	)
}

func (self ScShaErrorInvalidCanonicalSource) Is(target error) bool {
	return target == ErrScShaErrorInvalidCanonicalSource
}

type ScShaErrorInvalidManifest struct {
	Code    string
	Message string
}

func NewScShaErrorInvalidManifest(
	code string,
	message string,
) *ScShaError {
	return &ScShaError{err: &ScShaErrorInvalidManifest{
		Code:    code,
		Message: message}}
}

func (e ScShaErrorInvalidManifest) destroy() {
	FfiDestroyerString{}.Destroy(e.Code)
	FfiDestroyerString{}.Destroy(e.Message)
}

func (err ScShaErrorInvalidManifest) Error() string {
	return fmt.Sprint("InvalidManifest",
		": ",

		"Code=",
		err.Code,
		", ",
		"Message=",
		err.Message,
	)
}

func (self ScShaErrorInvalidManifest) Is(target error) bool {
	return target == ErrScShaErrorInvalidManifest
}

type ScShaErrorUnsupportedManifestSchema struct {
	Code    string
	Message string
}

func NewScShaErrorUnsupportedManifestSchema(
	code string,
	message string,
) *ScShaError {
	return &ScShaError{err: &ScShaErrorUnsupportedManifestSchema{
		Code:    code,
		Message: message}}
}

func (e ScShaErrorUnsupportedManifestSchema) destroy() {
	FfiDestroyerString{}.Destroy(e.Code)
	FfiDestroyerString{}.Destroy(e.Message)
}

func (err ScShaErrorUnsupportedManifestSchema) Error() string {
	return fmt.Sprint("UnsupportedManifestSchema",
		": ",

		"Code=",
		err.Code,
		", ",
		"Message=",
		err.Message,
	)
}

func (self ScShaErrorUnsupportedManifestSchema) Is(target error) bool {
	return target == ErrScShaErrorUnsupportedManifestSchema
}

type ScShaErrorDuplicateSource struct {
	Code    string
	Message string
}

func NewScShaErrorDuplicateSource(
	code string,
	message string,
) *ScShaError {
	return &ScShaError{err: &ScShaErrorDuplicateSource{
		Code:    code,
		Message: message}}
}

func (e ScShaErrorDuplicateSource) destroy() {
	FfiDestroyerString{}.Destroy(e.Code)
	FfiDestroyerString{}.Destroy(e.Message)
}

func (err ScShaErrorDuplicateSource) Error() string {
	return fmt.Sprint("DuplicateSource",
		": ",

		"Code=",
		err.Code,
		", ",
		"Message=",
		err.Message,
	)
}

func (self ScShaErrorDuplicateSource) Is(target error) bool {
	return target == ErrScShaErrorDuplicateSource
}

type ScShaErrorUnknownEdgeEndpoint struct {
	Code    string
	Message string
}

func NewScShaErrorUnknownEdgeEndpoint(
	code string,
	message string,
) *ScShaError {
	return &ScShaError{err: &ScShaErrorUnknownEdgeEndpoint{
		Code:    code,
		Message: message}}
}

func (e ScShaErrorUnknownEdgeEndpoint) destroy() {
	FfiDestroyerString{}.Destroy(e.Code)
	FfiDestroyerString{}.Destroy(e.Message)
}

func (err ScShaErrorUnknownEdgeEndpoint) Error() string {
	return fmt.Sprint("UnknownEdgeEndpoint",
		": ",

		"Code=",
		err.Code,
		", ",
		"Message=",
		err.Message,
	)
}

func (self ScShaErrorUnknownEdgeEndpoint) Is(target error) bool {
	return target == ErrScShaErrorUnknownEdgeEndpoint
}

type FfiConverterScShaError struct{}

var FfiConverterScShaErrorINSTANCE = FfiConverterScShaError{}

func (c FfiConverterScShaError) Lift(eb RustBufferI) *ScShaError {
	return LiftFromRustBuffer[*ScShaError](c, eb)
}

func (c FfiConverterScShaError) Lower(value *ScShaError) C.RustBuffer {
	return LowerIntoRustBuffer[*ScShaError](c, value)
}

func (c FfiConverterScShaError) LowerExternal(value *ScShaError) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[*ScShaError](c, value))
}

func (c FfiConverterScShaError) Read(reader io.Reader) *ScShaError {
	errorID := readUint32(reader)

	switch errorID {
	case 1:
		return &ScShaError{&ScShaErrorInvalidUtf8{
			Code:    FfiConverterStringINSTANCE.Read(reader),
			Message: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 2:
		return &ScShaError{&ScShaErrorInvalidDigest{
			Code:    FfiConverterStringINSTANCE.Read(reader),
			Message: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 3:
		return &ScShaError{&ScShaErrorInvalidCanonicalSource{
			Code:    FfiConverterStringINSTANCE.Read(reader),
			Message: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 4:
		return &ScShaError{&ScShaErrorInvalidManifest{
			Code:    FfiConverterStringINSTANCE.Read(reader),
			Message: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 5:
		return &ScShaError{&ScShaErrorUnsupportedManifestSchema{
			Code:    FfiConverterStringINSTANCE.Read(reader),
			Message: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 6:
		return &ScShaError{&ScShaErrorDuplicateSource{
			Code:    FfiConverterStringINSTANCE.Read(reader),
			Message: FfiConverterStringINSTANCE.Read(reader),
		}}
	case 7:
		return &ScShaError{&ScShaErrorUnknownEdgeEndpoint{
			Code:    FfiConverterStringINSTANCE.Read(reader),
			Message: FfiConverterStringINSTANCE.Read(reader),
		}}
	default:
		panic(fmt.Sprintf("Unknown error code %d in FfiConverterScShaError.Read()", errorID))
	}
}

func (c FfiConverterScShaError) Write(writer io.Writer, value *ScShaError) {
	switch variantValue := value.err.(type) {
	case *ScShaErrorInvalidUtf8:
		writeInt32(writer, 1)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Code)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Message)
	case *ScShaErrorInvalidDigest:
		writeInt32(writer, 2)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Code)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Message)
	case *ScShaErrorInvalidCanonicalSource:
		writeInt32(writer, 3)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Code)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Message)
	case *ScShaErrorInvalidManifest:
		writeInt32(writer, 4)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Code)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Message)
	case *ScShaErrorUnsupportedManifestSchema:
		writeInt32(writer, 5)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Code)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Message)
	case *ScShaErrorDuplicateSource:
		writeInt32(writer, 6)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Code)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Message)
	case *ScShaErrorUnknownEdgeEndpoint:
		writeInt32(writer, 7)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Code)
		FfiConverterStringINSTANCE.Write(writer, variantValue.Message)
	default:
		_ = variantValue
		panic(fmt.Sprintf("invalid error value `%v` in FfiConverterScShaError.Write", value))
	}
}

type FfiDestroyerScShaError struct{}

func (_ FfiDestroyerScShaError) Destroy(value *ScShaError) {
	switch variantValue := value.err.(type) {
	case ScShaErrorInvalidUtf8:
		variantValue.destroy()
	case ScShaErrorInvalidDigest:
		variantValue.destroy()
	case ScShaErrorInvalidCanonicalSource:
		variantValue.destroy()
	case ScShaErrorInvalidManifest:
		variantValue.destroy()
	case ScShaErrorUnsupportedManifestSchema:
		variantValue.destroy()
	case ScShaErrorDuplicateSource:
		variantValue.destroy()
	case ScShaErrorUnknownEdgeEndpoint:
		variantValue.destroy()
	default:
		_ = variantValue
		panic(fmt.Sprintf("invalid error value `%v` in FfiDestroyerScShaError.Destroy", value))
	}
}

type FfiConverterSequenceResolvedIncludeEdge struct{}

var FfiConverterSequenceResolvedIncludeEdgeINSTANCE = FfiConverterSequenceResolvedIncludeEdge{}

func (c FfiConverterSequenceResolvedIncludeEdge) Lift(rb RustBufferI) []ResolvedIncludeEdge {
	return LiftFromRustBuffer[[]ResolvedIncludeEdge](c, rb)
}

func (c FfiConverterSequenceResolvedIncludeEdge) Read(reader io.Reader) []ResolvedIncludeEdge {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]ResolvedIncludeEdge, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterResolvedIncludeEdgeINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceResolvedIncludeEdge) Lower(value []ResolvedIncludeEdge) C.RustBuffer {
	return LowerIntoRustBuffer[[]ResolvedIncludeEdge](c, value)
}

func (c FfiConverterSequenceResolvedIncludeEdge) LowerExternal(value []ResolvedIncludeEdge) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[[]ResolvedIncludeEdge](c, value))
}

func (c FfiConverterSequenceResolvedIncludeEdge) Write(writer io.Writer, value []ResolvedIncludeEdge) {
	if len(value) > math.MaxInt32 {
		panic("[]ResolvedIncludeEdge is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterResolvedIncludeEdgeINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceResolvedIncludeEdge struct{}

func (FfiDestroyerSequenceResolvedIncludeEdge) Destroy(sequence []ResolvedIncludeEdge) {
	for _, value := range sequence {
		FfiDestroyerResolvedIncludeEdge{}.Destroy(value)
	}
}

type FfiConverterSequenceResolvedTemplateNode struct{}

var FfiConverterSequenceResolvedTemplateNodeINSTANCE = FfiConverterSequenceResolvedTemplateNode{}

func (c FfiConverterSequenceResolvedTemplateNode) Lift(rb RustBufferI) []ResolvedTemplateNode {
	return LiftFromRustBuffer[[]ResolvedTemplateNode](c, rb)
}

func (c FfiConverterSequenceResolvedTemplateNode) Read(reader io.Reader) []ResolvedTemplateNode {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]ResolvedTemplateNode, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterResolvedTemplateNodeINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceResolvedTemplateNode) Lower(value []ResolvedTemplateNode) C.RustBuffer {
	return LowerIntoRustBuffer[[]ResolvedTemplateNode](c, value)
}

func (c FfiConverterSequenceResolvedTemplateNode) LowerExternal(value []ResolvedTemplateNode) ExternalCRustBuffer {
	return RustBufferFromC(LowerIntoRustBuffer[[]ResolvedTemplateNode](c, value))
}

func (c FfiConverterSequenceResolvedTemplateNode) Write(writer io.Writer, value []ResolvedTemplateNode) {
	if len(value) > math.MaxInt32 {
		panic("[]ResolvedTemplateNode is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterResolvedTemplateNodeINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceResolvedTemplateNode struct{}

func (FfiDestroyerSequenceResolvedTemplateNode) Destroy(sequence []ResolvedTemplateNode) {
	for _, value := range sequence {
		FfiDestroyerResolvedTemplateNode{}.Destroy(value)
	}
}

func CalculateCompositionHash(manifest ResolvedTemplateManifest) (CompositionHash, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[*ScShaError](FfiConverterScShaError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_sc_sha_go_fn_func_calculate_composition_hash(FfiConverterResolvedTemplateManifestINSTANCE.Lower(manifest), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue CompositionHash
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterCompositionHashINSTANCE.Lift(_uniffiRV), nil
	}
}

func CalculateHash(utf8FileBytes []byte) (TemplateHash, error) {
	_uniffiRV, _uniffiErr := rustCallWithError[*ScShaError](FfiConverterScShaError{}, func(_uniffiStatus *C.RustCallStatus) RustBufferI {
		return GoRustBuffer{
			inner: C.uniffi_sc_sha_go_fn_func_calculate_hash(FfiConverterBytesINSTANCE.Lower(utf8FileBytes), _uniffiStatus),
		}
	})
	if _uniffiErr != nil {
		var _uniffiDefaultValue TemplateHash
		return _uniffiDefaultValue, _uniffiErr
	} else {
		return FfiConverterTemplateHashINSTANCE.Lift(_uniffiRV), nil
	}
}
