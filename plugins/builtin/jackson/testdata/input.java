// Test input for jackson plugin
public class Pet {
    @JsonProperty("pet_name")
    private String name;

    @JsonIgnore
    private String internalId;
}
