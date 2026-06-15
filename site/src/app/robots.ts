import type { MetadataRoute } from 'next';

const SITE = 'https://nubjs.com';

export const dynamic = 'force-static';
export const revalidate = false;

/**
 * Allow all crawlers (including AI crawlers — the site deliberately publishes an
 * llms.txt index for them) and advertise the sitemap. The API/search route is
 * disallowed: it is a JSON endpoint, not indexable content.
 */
export default function robots(): MetadataRoute.Robots {
  return {
    rules: {
      userAgent: '*',
      allow: '/',
      disallow: '/api/',
    },
    sitemap: `${SITE}/sitemap.xml`,
    host: SITE,
  };
}
