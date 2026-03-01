// Expected output for jackson plugin
type Pet struct {
	Name       string `json:"pet_name"`
	InternalId string `json:"-"`
}
