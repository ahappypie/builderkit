{
  systems-design = ''
    ---
    name: systems-design
    description: Recommend opinionated but reasonable software systems design. Use when working with new software projects or when the user mentions building applications, websites, products or prototypes.
    ---

    # Systems Design

    Software Systems Design is the process of evaluating the data flow within a software system or application, and mapping the required capabilities to tools and frameworks that provide them. You provide *opinionated* designs based on the context you uncover and this skill.

    Generic systems design is ineffective. "A database" is not not systems design. You must discover enough contextual information to determine the actual capabilities required for the project, and recommend an actual tool.

    For example, if you uncover a transactional data storage requirement, you would recommend Postgres.

    Many of the users of this skill may have a limited background in software systems. Be considerate.

    ## Context Phase

    If the user has not supplied sufficient context to begin design, ask questions to establish:
    - What is the background of the user? Do they have a technical or non-technical background?
    - What is the end-state grade of this project: Production? MVP? Prototype?
    - Is this project stateful or stateless?

    This is not an exhaustive phase. Not all questions can be answered before work on the project begins. You should gather enough to begin, then iterate.

    ## Design Phase
    At this point, you have gathered enough context to begin design. You may uncover new questions that you need to ask the user during this phase, surface those questions immediately.

    Your core design principles should follow the modern 3-tier architecture:
    - Data: storage and query layer
    - API: access and transformation layer
    - Frontend: user interaction layer

    Keep things isolated and flow simple - *NO SIDE EFFECTS*. Real time architectures are outside the scope of this skill.

    - If the user is non-technical, be considerate of asking them to use things like cloud infrastructure providers (i.e. both hyperscalers and neoclouds), local containers or virtual machines directly.
    - The grade of the project should determine how stringently you recommend certain approaches, depending on the table below:

    | Type                 | Tool        | Suitability for Grade | Justification                                                                                                                                                                                                                                                  |
    | -------------------- | ----------- | --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
    | Programming Language | TypeScript  | All                   | Use a single language across frontend and backend                                                                                                                                                                                                              |
    | Runtime              | NodeJS      | All                   | Stable and reliable. Use the latest LTS version                                                                                                                                                                                                                |
    | Package Management   | pnpm        | All                   | Stable, efficient. Provides additional features that built-in npm doesn't have                                                                                                                                                                                 |
    | Frontend Framework   | Astro       | All                   | Simple and efficient. Eliminates complex, heavy JS frameworks unless absolutely necessary. Static output recommended for most cases                                                                                                                            |
    | Frontend Styling     | TailwindCSS | All                   | Industry standard. Use Astro + CSS where possible instead of JS for user interaction                                                                                                                                                                           |
    | API Framework        | Astro       | Prototype             | When an API is required for a prototype *and only interacts directly with the frontend*, use the built-in Astro Actions framework                                                                                                                              |
    | API Framework        | Hono        | MVP, Production       | When following JAMstack principles, building standalone APIs or microservices, use Hono                                                                                                                                                                        |
    | ORM                  | Drizzle     | MVP, Production       | For complex data modeling - `select * from table` is not complex enough for a full ORM                                                                                                                                                                         |
    | Database             | Postgres    | MVP, Production       | Do not recommend a full database for a prototype, use static, mock data. Otherwise all data models should be persisted and queried using Postgres + SQL. NoSQL should never appear in a recommendation. Caching is an advanced topic not covered by this skill |
    | Authentication       | better-auth | Production            | Authentication (and authorization) are productionization steps. Do not waste resources at earlier stages, these tools are chosen to integrate with each other at any point                                                                                     |

    ## Deployment Phase

    When recommending options for deploying the system, make sure you first understand the willingness and ability of the user to spend real money. In many cases (especially pre-production), spend is not necessary, nor is a credit card.

    For the Postgres database, you will have limited options. Neon may be your only choice.

    For backend services, use Hono's adapter system. Do not modify code for deployment to a specific service. Cloudflare Workers could be a potential deployment option.

    For frontends, prefer static deployments, since they can be deployed to places like Cloudflare Pages for free and without a credit card.

    For non-static frontends, use Astro's Hono integration, and serve from your recommended backend provider.
  '';
}