import { NgModule, Pipe, PipeTransform } from '@angular/core'
import Fuse from 'fuse.js'

import { MarketplacePkg } from '../types/marketplace-pkg'
import { MarketplaceManifest } from '../types/marketplace-manifest'
import { Emver } from '@start9labs/shared'

const defaultOps = {
  isCaseSensitive: false,
  includeScore: true,
  shouldSort: true,
  includeMatches: false,
  findAllMatches: false,
  minMatchCharLength: 1,
  location: 0,
  threshold: 0.6,
  distance: 100,
  useExtendedSearch: false,
  ignoreLocation: false,
  ignoreFieldNorm: false,
  keys: [
    'manifest.id',
    'manifest.title',
    'manifest.description.short',
    'manifest.description.long',
  ],
}

@Pipe({
  name: 'filterPackages',
})
export class FilterPackagesPipe implements PipeTransform {
  constructor(private readonly emver: Emver) {}

  transform(
    packages: MarketplacePkg[],
    query: string,
    category: string,
    local: Record<string, { manifest: MarketplaceManifest }> = {},
  ): MarketplacePkg[] {
    if (query) {
      const fuse = new Fuse(packages, defaultOps)

      return fuse.search(query).map(p => p.item)
    }

    if (category === 'updates') {
      return packages.filter(
        ({ manifest }) =>
          local[manifest.id] &&
          this.emver.compare(
            manifest.version,
            local[manifest.id].manifest.version,
          ) === 1,
      )
    }

    const pkgsToSort = packages.filter(
      p => category === 'all' || p.categories.includes(category),
    )
    const fuse = new Fuse(pkgsToSort, { ...defaultOps, threshold: 1 })

    return fuse
      .search(category !== 'all' ? category || '' : 'bit')
      .map(p => p.item)
  }
}

@NgModule({
  declarations: [FilterPackagesPipe],
  exports: [FilterPackagesPipe],
})
export class FilterPackagesPipeModule {}
