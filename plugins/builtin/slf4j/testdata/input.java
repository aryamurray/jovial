// Test input for slf4j plugin
private static final Logger log = LoggerFactory.getLogger(PetService.class);
log.info("Finding pet with id: {}", id);
log.error("Pet not found", exception);
