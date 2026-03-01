// Test input for spring-web plugin
@RestController
@RequestMapping("/api/pets")
public class PetController {
    @GetMapping("/{id}")
    public ResponseEntity<Pet> getPet(@PathVariable Long id) {
        return ResponseEntity.ok(petService.findById(id));
    }
}
