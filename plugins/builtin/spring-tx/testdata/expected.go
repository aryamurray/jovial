// Expected output for spring-tx plugin
func (s *AccountService) TransferMoney(fromId, toId int64, amount float64) error {
	tx := s.db.Begin()
	defer func() {
		if r := recover(); r != nil {
			tx.Rollback()
		}
	}()
	// TODO: implement
	return tx.Commit().Error
}
