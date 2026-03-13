# data

Local developer working directory for large inputs, scratch outputs, and non-runtime datasets.

Most contents under this directory should remain gitignored. Keep committed documentation under `data/spec/`, allow only small explicit fixtures like `data/landmarks/*.csv`, and avoid teaching runtime components to rely on local raw data.
