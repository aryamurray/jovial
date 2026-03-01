// Expected output for spring-data plugin
package models

import "gorm.io/gorm"

type Pet struct {
	gorm.Model
	Name    string `gorm:"not null"`
	OwnerID uint
	Owner   Owner
}
