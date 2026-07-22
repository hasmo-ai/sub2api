//go:build unit

package handler

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/Wei-Shaw/sub2api/internal/config"
	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/require"
)

func TestDepositHandler_GetDepositConfig_Enabled(t *testing.T) {
	gin.SetMode(gin.TestMode)

	h := NewDepositHandler(&config.Config{
		SolanaDeposit: config.SolanaDepositConfig{
			Enabled:       true,
			Cluster:       "devnet",
			ProgramID:     "Depos1tProgram1111111111111111111111111111",
			USDCMint:      "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
			MinDepositUSD: 1.5,
		},
	})

	recorder := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(recorder)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/user/deposit-config", nil)

	h.GetDepositConfig(c)

	require.Equal(t, http.StatusOK, recorder.Code)

	var resp struct {
		Code int `json:"code"`
		Data struct {
			Enabled       bool    `json:"enabled"`
			Cluster       string  `json:"cluster"`
			ProgramID     string  `json:"program_id"`
			USDCMint      string  `json:"usdc_mint"`
			MinDepositUSD float64 `json:"min_deposit_usd"`
		} `json:"data"`
	}
	require.NoError(t, json.Unmarshal(recorder.Body.Bytes(), &resp))
	require.Equal(t, 0, resp.Code)
	require.True(t, resp.Data.Enabled)
	require.Equal(t, "devnet", resp.Data.Cluster)
	require.Equal(t, "Depos1tProgram1111111111111111111111111111", resp.Data.ProgramID)
	require.Equal(t, "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU", resp.Data.USDCMint)
	require.Equal(t, 1.5, resp.Data.MinDepositUSD)
}

func TestDepositHandler_GetDepositConfig_Disabled(t *testing.T) {
	gin.SetMode(gin.TestMode)

	h := NewDepositHandler(&config.Config{})

	recorder := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(recorder)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/user/deposit-config", nil)

	h.GetDepositConfig(c)

	require.Equal(t, http.StatusOK, recorder.Code)

	var resp struct {
		Code int `json:"code"`
		Data struct {
			Enabled       bool    `json:"enabled"`
			Cluster       string  `json:"cluster"`
			ProgramID     string  `json:"program_id"`
			USDCMint      string  `json:"usdc_mint"`
			MinDepositUSD float64 `json:"min_deposit_usd"`
		} `json:"data"`
	}
	require.NoError(t, json.Unmarshal(recorder.Body.Bytes(), &resp))
	require.Equal(t, 0, resp.Code)
	require.False(t, resp.Data.Enabled)
	require.Empty(t, resp.Data.Cluster)
	require.Empty(t, resp.Data.ProgramID)
	require.Empty(t, resp.Data.USDCMint)
	require.Zero(t, resp.Data.MinDepositUSD)
}
