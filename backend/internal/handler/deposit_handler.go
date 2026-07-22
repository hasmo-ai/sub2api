package handler

import (
	"github.com/Wei-Shaw/sub2api/internal/config"
	"github.com/Wei-Shaw/sub2api/internal/pkg/response"

	"github.com/gin-gonic/gin"
)

// DepositHandler handles on-chain deposit-related requests
type DepositHandler struct {
	cfg *config.Config
}

// NewDepositHandler creates a new DepositHandler
func NewDepositHandler(cfg *config.Config) *DepositHandler {
	return &DepositHandler{cfg: cfg}
}

// DepositConfigResponse represents the public on-chain deposit configuration
type DepositConfigResponse struct {
	Enabled       bool    `json:"enabled"`
	Cluster       string  `json:"cluster"`
	ProgramID     string  `json:"program_id"`
	USDCMint      string  `json:"usdc_mint"`
	MinDepositUSD float64 `json:"min_deposit_usd"`
}

// GetDepositConfig returns the public on-chain deposit configuration
// GET /api/v1/user/deposit-config
func (h *DepositHandler) GetDepositConfig(c *gin.Context) {
	cfg := h.cfg.SolanaDeposit
	response.Success(c, DepositConfigResponse{
		Enabled:       cfg.Enabled,
		Cluster:       cfg.Cluster,
		ProgramID:     cfg.ProgramID,
		USDCMint:      cfg.USDCMint,
		MinDepositUSD: cfg.MinDepositUSD,
	})
}
