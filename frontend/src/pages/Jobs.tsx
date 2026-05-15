import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery } from 'react-query';
import {
  Box,
  Paper,
  CircularProgress,
  Chip,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  SelectChangeEvent,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TableContainer,
  Typography,
} from '@mui/material';
import { jobsApi, Job, JobStatus } from '../api/apiClient';
import PageHeader from '../components/PageHeader';

function statusColor(status: Job['status']) {
  switch (status) {
    case 'done':
      return 'success';
    case 'dead':
      return 'error';
    case 'processing':
      return 'info';
    case 'pending':
      return 'warning';
    default:
      return 'default';
  }
}

function Jobs(): React.ReactElement {
  const navigate = useNavigate();
  const [statusFilter, setStatusFilter] = useState<JobStatus | 'all'>('all');

  const { data, isLoading } = useQuery(
    ['jobs', statusFilter],
    () => jobsApi.list(statusFilter === 'all' ? { limit: 100 } : { status: statusFilter, limit: 100 }),
    { refetchInterval: 5000 },
  );

  return (
    <Box>
      <PageHeader title="Jobs" subtitle="All ingested events and their delivery status" />

      <Paper sx={{ p: 2, mb: 2 }}>
        <FormControl size="small" sx={{ minWidth: 200 }}>
          <InputLabel id="status-label">Status</InputLabel>
          <Select
            labelId="status-label"
            label="Status"
            value={statusFilter}
            onChange={(e: SelectChangeEvent) => setStatusFilter(e.target.value as JobStatus | 'all')}
          >
            <MenuItem value="all">All</MenuItem>
            <MenuItem value="pending">Pending</MenuItem>
            <MenuItem value="processing">Processing</MenuItem>
            <MenuItem value="done">Delivered</MenuItem>
            <MenuItem value="dead">Dead-lettered</MenuItem>
          </Select>
        </FormControl>
      </Paper>

      {isLoading ? (
        <Box display="flex" justifyContent="center" mt={4}>
          <CircularProgress />
        </Box>
      ) : (
        <TableContainer component={Paper}>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>Status</TableCell>
                <TableCell>Route</TableCell>
                <TableCell>Source path</TableCell>
                <TableCell align="right">Attempts</TableCell>
                <TableCell>Last error</TableCell>
                <TableCell>Updated</TableCell>
                <TableCell>ID</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {data && data.length > 0 ? (
                data.map((job) => (
                  <TableRow
                    key={job.id}
                    hover
                    sx={{ cursor: 'pointer' }}
                    onClick={() => navigate(`/jobs/${job.id}`)}
                  >
                    <TableCell>
                      <Chip label={job.status} size="small" color={statusColor(job.status) as any} />
                    </TableCell>
                    <TableCell>{job.route_id}</TableCell>
                    <TableCell>
                      <Typography variant="body2" fontFamily="monospace">
                        {job.source_path}
                      </Typography>
                    </TableCell>
                    <TableCell align="right">{job.attempts}</TableCell>
                    <TableCell>
                      <Typography variant="caption" color="error.main" noWrap sx={{ maxWidth: 300, display: 'block' }}>
                        {job.last_error}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="caption">{new Date(job.updated_at).toLocaleString()}</Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="caption" fontFamily="monospace">
                        {job.id.slice(0, 8)}
                      </Typography>
                    </TableCell>
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell colSpan={7}>
                    <Typography align="center" color="text.secondary" sx={{ py: 2 }}>
                      No jobs match this filter.
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

export default Jobs;
