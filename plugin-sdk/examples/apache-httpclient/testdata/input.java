// Apache HttpClient plugin test input
CloseableHttpClient client = HttpClients.createDefault();
HttpGet request = new HttpGet("https://api.example.com/data");
CloseableHttpResponse response = client.execute(request);
