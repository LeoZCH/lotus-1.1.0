package sectorstorage

import (
	"context"

	"golang.org/x/xerrors"

	"github.com/filecoin-project/go-state-types/abi"

	"github.com/filecoin-project/lotus/extern/sector-storage/sealtasks"
	"github.com/filecoin-project/lotus/extern/sector-storage/stores"
)

type allocSelector struct {
	index stores.SectorIndex
	alloc stores.SectorFileType
	ptype stores.PathType
}

func newAllocSelector(index stores.SectorIndex, alloc stores.SectorFileType, ptype stores.PathType) *allocSelector {
	return &allocSelector{
		index: index,
		alloc: alloc,
		ptype: ptype,
	}
}

func (s *allocSelector) Ok(ctx context.Context, task sealtasks.TaskType, spt abi.RegisteredSealProof, whnd *workerHandle) (bool, error) {
	tasks, err := whnd.w.TaskTypes(ctx)
	if err != nil {
		return false, xerrors.Errorf("getting supported worker task types: %w", err)
	}
	if _, supported := tasks[task]; !supported {
		return false, nil
	}

	paths, err := whnd.w.Paths(ctx)
	if err != nil {
		return false, xerrors.Errorf("getting worker paths: %w", err)
	}

	have := map[stores.ID]struct{}{}
	for _, path := range paths {
		have[path.ID] = struct{}{}
	}

	best, err := s.index.StorageBestAlloc(ctx, s.alloc, spt, s.ptype)
	if err != nil {
		return false, xerrors.Errorf("finding best alloc storage: %w", err)
	}

	for _, info := range best {
		if _, ok := have[info.ID]; ok {
			return true, nil
		}
	}

	return false, nil
}

func (s *allocSelector) Cmp(ctx context.Context, task sealtasks.TaskType, a, b *workerHandle) (bool, error) {
	return a.utilization() < b.utilization(), nil
}

var _ WorkerSelector = &allocSelector{}

func (s *allocSelector) FindDataWoker(ctx context.Context, task sealtasks.TaskType, sid abi.SectorID, spt abi.RegisteredSealProof, whnd *workerHandle) bool {
	paths, err := whnd.w.Paths(ctx)
	if err != nil {
		return false
	}

	have := map[stores.ID]struct{}{}
	for _, path := range paths {
		have[path.ID] = struct{}{}
	}

	var ft stores.SectorFileType
	switch task {
	case sealtasks.TTAddPiece:
		ft = 0
	case sealtasks.TTPreCommit1:
		ft = stores.FTUnsealed
	case sealtasks.TTPreCommit2:
		ft = stores.FTCache | stores.FTSealed
	case sealtasks.TTCommit1:
		ft = stores.FTCache | stores.FTSealed
	case sealtasks.TTCommit2:
		ft = stores.FTCache | stores.FTSealed
	case sealtasks.TTFetch:
		ft = stores.FTUnsealed | stores.FTCache | stores.FTSealed
	case sealtasks.TTFinalize:
		ft = stores.FTCache | stores.FTSealed
	}

	find, err := s.index.StorageFindSector(ctx, sid, ft, spt, false)
	if err != nil {
		return false
	}

	for _, info := range find {
		if _, ok := have[info.ID]; ok {
			return true
		}
	}

	return false
}
