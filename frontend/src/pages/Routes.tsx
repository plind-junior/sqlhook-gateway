import React from 'react';
import { useQuery } from 'react-query';
import {
  Box,
  Paper,
  CircularProgress,
  Typography,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TableContainer,
  Alert,
} from '@mui/material';
import { routesApi } from '../api/apiClient';
import PageHeader from '../components/PageHeader';

function Routes(): React.ReactElement {
  const { data, isLoading } = useQuery('routes', routesApi.list);

  return (
    <Box>
      <PageHeader title="Routes" subtitle="Read-only view of the loaded route configuration" />

      <Alert severity="info" sx={{ mb: 2 }}>
        Routes are defined in <code>config.yaml</code>. To change them, edit the file and restart sqlhook.
      </Alert>

      {isLoading ? (
        <Box display="flex" justifyContent="center" mt={4}>
          <CircularProgress />
        </Box>
      ) : (
        <TableContainer component={Paper}>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>ID</TableCell>
                <TableCell>Source path</TableCell>
                <TableCell>Destination</TableCell>
                <TableCell>Signature header</TableCell>
                <TableCell align="right">Timeout (ms)</TableCell>
                <TableCell align="right">Max attempts</TableCell>
                <TableCell align="right">Backoff (ms)</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {data && data.length > 0 ? (
                data.map((r) => (
                  <TableRow key={r.id}>
                    <TableCell>{r.id}</TableCell>
                    <TableCell>
                      <Typography variant="body2" fontFamily="monospace">
                        {r.source_path}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="caption" fontFamily="monospace">
                        {r.destination_url}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="caption" fontFamily="monospace">
                        {r.signature_header}
                      </Typography>
                    </TableCell>
                    <TableCell align="right">{r.timeout_ms}</TableCell>
                    <TableCell align="right">{r.max_attempts}</TableCell>
                    <TableCell align="right">
                      {r.initial_backoff_ms} → {r.max_backoff_ms}
                    </TableCell>
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell colSpan={7}>
                    <Typography align="center" color="text.secondary" sx={{ py: 2 }}>
                      No routes configured.
                    </Typography>
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      )}
    </Box>
  );
}

export default Routes;
