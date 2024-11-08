import { useQuery } from "@tanstack/react-query";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";

const apiUrl = import.meta.env.VITE_APP_API_URL;
if (!apiUrl) {
	throw new Error("VITE_APP_API_URL is not set");
}

const fetchApi = async (search: string, page: number) => {
	const response = await fetch(
		`${apiUrl}/api/search?query=${search}&page=${page}`,
	);
	return response.json() as Promise<
		Array<{
			title: string;
			link: string;
			snippet: string;
			source: string;
			score: number;
			breadcrumbs: Array<{
				text: string;
				url: string;
			}> | null;
			favicon_url: string | null;
			site_name: string | null;
		}>
	>;
};

const useUrl = () => {
	const [page, setStatePage] = useState(1);
	const [search, setStateSearch] = useState("");

	useEffect(() => {
		const urlParams = new URLSearchParams(window.location.search);
		const page = urlParams.get("page");
		const search = urlParams.get("search");
		if (page) {
			setStatePage(Number.parseInt(page));
		}
		if (search) {
			setStateSearch(search);
		}
	}, []);

	const setPage = (newPage: number) => {
		setStatePage(newPage);
		const urlParams = new URLSearchParams(window.location.search);
		urlParams.set("page", newPage.toString());
		urlParams.set("search", search);
		window.history.pushState({}, "", `?${urlParams.toString()}`);
	};
	const setSearch = (newSearch: string) => {
		setStateSearch(newSearch);
		const urlParams = new URLSearchParams(window.location.search);
		urlParams.set("page", page.toString());
		urlParams.set("search", newSearch);
		window.history.pushState({}, "", `?${urlParams.toString()}`);
	};

	return { page, search, setPage, setSearch };
};

const useSearchQuery = (search: string, page: number) => {
	const [debouncedSearch, setDebouncedSearch] = useState(search);

	useEffect(() => {
		let handler: ReturnType<typeof setTimeout> | undefined;

		if (search) {
			handler = setTimeout(() => {
				setDebouncedSearch(search);
			}, 300);
		}

		return () => {
			if (handler) clearTimeout(handler);
		};
	}, [search]);

	return useQuery({
		queryKey: ["search", page, debouncedSearch],
		queryFn: () => fetchApi(debouncedSearch, page),
		enabled: Boolean(debouncedSearch),
	});
};

function App() {
	const { page, search, setPage, setSearch } = useUrl();
	const { data, isFetching, error, isError } = useSearchQuery(search, page);
	const [lastScrollY, setLastScrollY] = useState(0);
	const inputRef = useRef<HTMLInputElement>(null);
	const searchBarRef = useRef<HTMLDivElement>(null);

	// Scroll handling for hiding/showing search bar
	useEffect(() => {
		const scrollHandler = () => {
			const currentScrollY = window.scrollY;
			if (searchBarRef.current) {
				searchBarRef.current.style.transform =
					currentScrollY > lastScrollY ? "translateY(-150%)" : "translateY(0)";
			}
			setLastScrollY(currentScrollY);
		};

		window.addEventListener("scroll", scrollHandler);
		return () => window.removeEventListener("scroll", scrollHandler);
	}, [lastScrollY]);

	const handleSearch = (event: React.ChangeEvent<HTMLInputElement>) => {
		const value = event.target.value;
		setSearch(value); // Mise à jour immédiate du terme de recherche
		if (value.length === 0) {
			setPage(1); // Réinitialiser à la première page si la recherche est vide
		}
	};

	const handlePagination = (type: "next" | "prev" | "first") => {
		if (type === "next") {
			setPage(page + 1);
		} else if (type === "prev" && page > 1) {
			setPage(page - 1);
		} else if (type === "first") {
			setPage(1);
		}
	};

	return (
		<div
			style={{
				display: "flex",
				flexDirection: "column",
				alignItems: "center",
				minHeight: "100dvh",
				padding: "1rem",
				fontFamily: "sans-serif",
			}}
		>
			<div
				style={{
					display: "flex",
					justifyContent: "center",
					alignItems: "center",
					gap: "1rem",
					position: "sticky",
					top: "1rem",
					backgroundColor: "rgba(255,255,255,0.5)",
					backdropFilter: "blur(10px)",
					width: "100%",
					padding: "2rem",
					borderRadius: "25px",
					border: "1px solid rgba(0,0,0,0.1)",
					boxShadow: "0 0 10px rgba(0,0,0,0.1)",
					zIndex: 1,
				}}
				ref={searchBarRef}
			>
				<input
					ref={inputRef}
					// biome-ignore lint/a11y/noAutofocus: <explanation>
					autoFocus
					autoComplete="off"
					inputMode="search"
					id="search"
					name="search"
					type="search"
					placeholder="Search for..."
					value={search}
					onChange={handleSearch}
					onKeyDown={(event: KeyboardEvent<HTMLInputElement>) => {
						if (event.key === "Enter" && event.currentTarget.value) {
							event.preventDefault();
							setSearch(search);
						}
					}}
				/>
				<button
					type="button"
					disabled={isFetching}
					onClick={() => handlePagination("prev")}
				>
					Prev
				</button>
				<button
					type="button"
					disabled={isFetching}
					onClick={() => handlePagination("next")}
				>
					Next
				</button>
				<button
					type="button"
					disabled={isFetching}
					onClick={() => handlePagination("first")}
				>
					First
				</button>
			</div>

			<div>
				{isFetching && (
					<p
						style={{
							position: "fixed",
							top: "50%",
							left: "50%",
							transform: "translateX(-50%) translateY(-50%)",
						}}
					>
						...
					</p>
				)}
				{isError && (
					<p
						style={{
							position: "fixed",
							top: "50%",
							left: "50%",
							transform: "translateX(-50%) translateY(-50%)",
						}}
					>
						Error: {error?.message}
					</p>
				)}
				{data && data.length > 0 && (
					<ul
						style={{
							marginTop: "25px",
							listStyleType: "none",
							display: "flex",
							flexDirection: "column",
							gap: "5px",
						}}
					>
						{data.map((item) => (
							<li
								style={{
									borderRadius: "5px",
									textDecoration: "none",
									display: "flex",
									flexDirection: "column",
									gap: "5px",
								}}
								key={JSON.stringify({ item })}
							>
								<a
									style={{
										position: "relative",
										textDecoration: "none",
										display: "flex",
										gap: "10px",
										padding: "15px",
									}}
									href={item.link.trim()}
									// target="_blank"
									// rel="noreferrer"
								>
									<div
										style={{
											display: "flex",
											gap: "3px",
											flexDirection: "column",
										}}
									>
										<div
											style={{
												display: "flex",
												gap: "10px",
											}}
										>
											<div
												style={{
													display: "flex",
													gap: "10px",
													alignItems: "center",
												}}
											>
												{item.favicon_url && (
													<div
														style={{
															borderRadius: "25%",
															overflow: "hidden",
															width: "20px",
															height: "20px",
															aspectRatio: "1/1",
															filter:
																"drop-shadow(0px 2px 1px rgba(0, 0, 0, 0.2))",
														}}
													>
														<img
															src={item.favicon_url.trim()}
															alt={item.site_name ?? item.title.trim()}
															width={20}
															height={20}
															style={{
																objectFit: "contain",
															}}
														/>
													</div>
												)}
											</div>
											<div>
												<div
													style={{
														display: "flex",
														flexDirection: "column",
														fontSize: "0.75rem",
													}}
												>
													{item.site_name && <p>{item.site_name}</p>}
													{item.breadcrumbs && item.breadcrumbs.length > 0 && (
														<ul
															style={{
																display: "flex",
																gap: "0.15rem",
																listStyleType: "none",
															}}
														>
															{item.breadcrumbs.map((breadcrumb) => (
																<li key={breadcrumb.url}>
																	<a href={breadcrumb.url}>
																		{breadcrumb.text} /
																	</a>
																</li>
															))}
														</ul>
													)}
												</div>
											</div>
										</div>
										<p
											style={{
												fontSize: "1.4rem",
											}}
										>
											{item.title}
										</p>
										<p
											style={{
												cursor: "text",
											}}
										>
											{item.snippet}
										</p>
									</div>
									<p
										style={{
											position: "absolute",
											top: "10px",
											right: "10px",
											fontSize: "12px",
											width: "15px",
											height: "15px",
											display: "flex",
											alignItems: "center",
											justifyContent: "center",
											borderRadius: "50%",
											backgroundColor: "rgba(0, 0, 0, 0.1)",
											lineHeight: "1",
										}}
										title={item.source}
									>
										{/* i */}
										{item.source}
									</p>
								</a>
							</li>
						))}
					</ul>
				)}
			</div>
		</div>
	);
}

export default App;
