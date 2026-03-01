// Apache HttpClient plugin test output
package example

import "net/http"

func FetchData() (*http.Response, error) {
	client := &http.Client{}
	req, err := http.NewRequest("GET", "https://api.example.com/data", nil)
	if err != nil {
		return nil, err
	}
	return client.Do(req)
}
