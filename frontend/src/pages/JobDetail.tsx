import React from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from 'react-query';
import {
  Box,
  Paper,
  Typography,
  CircularProgress,
  Chip,
  Grid,
  Button,
  Divider,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TableContainer,
  Alert,
} from '@mui/material';
import { useSnackbar } from 'notistack';
import { jobsApi, Job } from '../api/apiClient';
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

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <Grid item xs={12} sm={6}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="body2" fontFamily="monospace" sx={{ wordBreak: 'break-all' }}>
        {children}
      </Typography>
    </Grid>
  );
}

function JobDetail(): React.ReactElement {
  const { id = '' } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { enqueueSnackbar } = useSnackbar();

  const jobQuery = useQuery(['job', id], () => jobsApi.get(id), {
    refetchInterval: 5000,
  });
  const deliveriesQuery = useQuery(
    ['deliveries', id],
    () => jobsApi.deliveries(id),
    { refetchInterval: 5000 },
  );

  const replay = useMutation(() => jobsApi.replay(id), {
    onSuccess: () => {
      enqueueSnackbar('Job re-queued.', { variant: 'success' });
      queryClient.invalidateQueries(['job', id]);
      queryClient.invalidateQueries('recent-jobs');
      queryClient.invalidateQueries('stats');
    },
    onError: (e: any) => {
      const msg = e?.response?.data?.error ?? e?.message ?? 'replay failed';
      enqueueSnackbar(`Replay failed: ${msg}`, { variant: 'error' });
    },
  });

  if (jobQuery.isLoading) {
    return (
      <Box display="flex" justifyContent="center" mt={4}>
        <CircularProgress />
      </Box>
    );
  }

  if (jobQuery.isError || !jobQuery.data) {
    return (
      <Box>
        <PageHeader title="Job not found" />
        <Button onClick={() => navigate('/jobs')}>Back to jobs</Button>
      </Box>
    );
  }

  const job = jobQuery.data;
  const deliveries = deliveriesQuery.data ?? [];

  return (
    <Box>
      <PageHeader
        title="Job"
        subtitle={job.id}
        action={
          job.status === 'dead' ? (
            <Button
              variant="contained"
              color="warning"
              onClick={() => replay.mutate()}
              disabled={replay.isLoading}
            >
              {replay.isLoading ? 'Replaying...' : 'Replay'}
            </Button>
          ) : undefined
        }
      />

      <Paper sx={{ p: 2, mb: 2 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Chip label={job.status} color={statusColor(job.status) as any} />
          <Typography variant="body2" color="text.secondary">
            attempts: {job.attempts}
          </Typography>
        </Box>
        <Grid container spacing={2}>
          <Field label="Route">{job.route_id}</Field>
          <Field label="Source path">{job.source_path}</Field>
          <Field label="Created at">{new Date(job.created_at).toLocaleString()}</Field>
          <Field label="Updated at">{new Date(job.updated_at).toLocaleString()}</Field>
          <Field label="Visible at">{new Date(job.visible_at).toLocaleString()}</Field>
          <Field label="Last response code">{job.last_response_code ?? '—'}</Field>
        </Grid>
        {job.last_error && (
          <Alert severity="error" sx={{ mt: 2 }}>
            <Typography variant="caption" fontFamily="monospace">
              {job.last_error}
            </Typography>
          </Alert>
        )}
      </Paper>

      <Typography variant="h6" gutterBottom>
        Delivery attempts
      </Typography>
      <TableContainer component={Paper}>
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell>#</TableCell>
              <TableCell>Outcome</TableCell>
              <TableCell align="right">HTTP</TableCell>
              <TableCell align="right">Duration (ms)</TableCell>
              <TableCell>Error / Response</TableCell>
              <TableCell>Time</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {deliveries.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6}>
                  <Typography align="center" color="text.secondary" sx={{ py: 2 }}>
                    No delivery attempts yet.
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              deliveries.map((d) => (
                <TableRow key={d.id}>
                  <TableCell>{d.attempt}</TableCell>
                  <TableCell>
                    <Chip
                      label={d.success ? 'success' : 'failure'}
                      size="small"
                      color={d.success ? 'success' : 'error'}
                    />
                  </TableCell>
                  <TableCell align="right">{d.response_code ?? '—'}</TableCell>
                  <TableCell align="right">{d.duration_ms}</TableCell>
                  <TableCell>
                    <Typography variant="caption" fontFamily="monospace" sx={{ wordBreak: 'break-word' }}>
                      {d.error ?? d.response_body ?? '—'}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Typography variant="caption">{new Date(d.created_at).toLocaleString()}</Typography>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </TableContainer>

      {deliveries.length > 0 && deliveries[0].transformed_payload && (
        <Box sx={{ mt: 2 }}>
          <Typography variant="subtitle2" gutterBottom>
            Last transformed payload
          </Typography>
          <Paper sx={{ p: 2, bgcolor: '#1e1e1e', color: '#d4d4d4' }}>
            <pre style={{ margin: 0, whiteSpace: 'pre-wrap', fontSize: 13 }}>
              {(() => {
                try {
                  return JSON.stringify(JSON.parse(deliveries[0].transformed_payload!), null, 2);
                } catch {
                  return deliveries[0].transformed_payload;
                }
              })()}
            </pre>
          </Paper>
        </Box>
      )}

      <Divider sx={{ my: 2 }} />
      <Button onClick={() => navigate('/jobs')}>Back to jobs</Button>
    </Box>
  );
}

export default JobDetail;
