import { useQuery } from "@tanstack/react-query";
import {
	type KeyboardEvent,
	Suspense,
	useEffect,
	useRef,
	useState,
} from "react";
import PWABadge from "./PWABadge";

const apiUrl = import.meta.env.VITE_APP_API_URL;
if (!apiUrl) {
	throw new Error("VITE_APP_API_URL is not set");
}

const fetchApi = async (
	search: string,
	page: number,
	dateRange?: string,
	region?: string,
	language?: string,
) => {
	const params = new URLSearchParams({
		query: search,
		page: page.toString(),
	});

	if (dateRange) params.append("date_range", dateRange);
	if (region) params.append("region", region);
	if (language) params.append("language", language);

	const response = await fetch(`${apiUrl}/api/search?${params.toString()}`);
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

const fetchAutocomplete = async (search: string) => {
	const response = await fetch(`${apiUrl}/api/autocomplete?query=${search}`);
	return response.json() as Promise<string[]>;
};

const fetchQuickAnswers = async (search: string) => {
	const response = await fetch(`${apiUrl}/api/quick-answers?query=${search}`);
	return response.json() as Promise<
		ReadonlyArray<{
			answer_type: string;
			term: string;
			definition: string;
			source: string;
		}>
	>;
};

const useUrl = () => {
	const [page, setStatePage] = useState(1);
	const [search, setStateSearch] = useState("");
	const [dateRange, setStateDate] = useState("");
	const [region, setStateRegion] = useState("");
	const [language, setStateLanguage] = useState("");

	useEffect(() => {
		const urlParams = new URLSearchParams(window.location.search);
		const page = urlParams.get("page");
		const search = urlParams.get("search");
		const date = urlParams.get("date_range");
		const region = urlParams.get("region");
		const language = urlParams.get("language");

		if (page) setStatePage(Number.parseInt(page));
		if (search) setStateSearch(search);
		if (date) setStateDate(date);
		if (region) setStateRegion(region);
		if (language) setStateLanguage(language);
	}, []);

	const updateUrl = (updates: Record<string, string>) => {
		const urlParams = new URLSearchParams(window.location.search);
		for (const key of urlParams.keys()) {
			if (!(key in updates)) {
				urlParams.delete(key);
			}
		}
		window.history.pushState({}, "", `?${urlParams.toString()}`);
	};

	const setPage = (newPage: number) => {
		setStatePage(newPage);
		updateUrl({ page: newPage.toString(), search });
	};

	const setSearch = (newSearch: string) => {
		setStateSearch(newSearch);
		const urlParams = new URLSearchParams(window.location.search);

		if (newSearch === "") {
			urlParams.delete("search");
			urlParams.delete("page");
		} else {
			urlParams.set("search", newSearch);
			urlParams.set("page", page.toString());
		}

		window.history.pushState({}, "", `?${urlParams.toString()}`);
	};

	const setDateRange = (newDate: string) => {
		setStateDate(newDate);
		updateUrl({ date_range: newDate });
	};

	const setRegion = (newRegion: string) => {
		setStateRegion(newRegion);
		updateUrl({ region: newRegion });
	};

	const setLanguage = (newLanguage: string) => {
		setStateLanguage(newLanguage);
		updateUrl({ language: newLanguage });
	};

	return {
		page,
		search,
		dateRange,
		region,
		language,
		setPage,
		setSearch,
		setDateRange,
		setRegion,
		setLanguage,
	};
};

const useSearchQuery = (
	search: string,
	page: number,
	dateRange: string,
	region: string,
	language: string,
) => {
	const [debouncedSearch, setDebouncedSearch] = useState(search);

	useEffect(() => {
		const handler = setTimeout(() => {
			setDebouncedSearch(search);
		}, 300);

		return () => {
			if (handler) clearTimeout(handler);
		};
	}, [search]);

	return useQuery({
		queryKey: ["search", page, debouncedSearch, dateRange, region, language],
		queryFn: () => fetchApi(debouncedSearch, page, dateRange, region, language),
		enabled: Boolean(debouncedSearch),
	});
};

const useAutocompleteQuery = (search: string, enabled: boolean) => {
	return useQuery({
		queryKey: ["autocomplete", search],
		queryFn: () => fetchAutocomplete(search),
		enabled: enabled && search.length > 2,
	});
};

const useQuickAnswersQuery = (search: string) => {
	return useQuery({
		queryKey: ["quickAnswers", search],
		queryFn: () => fetchQuickAnswers(search),
		enabled: Boolean(search),
	});
};

function PopoverConfig({
	dateRange,
	setDateRange,
	region,
	setRegion,
	language,
	setLanguage,
	isFetching,
	handlePagination,
	popoverRef, // Ajout de la ref en prop
}: {
	dateRange: string;
	setDateRange: (value: string) => void;
	region: string;
	setRegion: (value: string) => void;
	language: string;
	setLanguage: (value: string) => void;
	page: number;
	isFetching: boolean;
	handlePagination: (type: "next" | "prev" | "first") => void;
	popoverRef: React.RefObject<HTMLDivElement>;
}) {
	return (
		<div
			id="config"
			ref={popoverRef}
			style={{
				position: "absolute",
				top: "100%",
				right: "0",
				marginTop: "10px",
				backgroundColor: "white",
				padding: "1rem",
				borderRadius: "10px",
				boxShadow: "0 4px 6px rgba(0,0,0,0.1)",
				border: "1px solid rgba(0,0,0,0.1)",
				zIndex: 10,
				display: "flex",
				flexDirection: "column",
				gap: "1rem",
				minWidth: "250px",
				pointerEvents: "auto",
			}}
		>
			<select
				value={dateRange}
				onChange={(e) => setDateRange(e.target.value)}
				style={{
					padding: "8px",
					borderRadius: "5px",
					border: "1px solid rgba(0,0,0,0.1)",
				}}
			>
				<option value="">Toute date</option>
				<option value="day">24h</option>
				<option value="week">Semaine</option>
				<option value="month">Mois</option>
				<option value="year">Année</option>
			</select>

			<select
				value={region}
				onChange={(e) => setRegion(e.target.value)}
				style={{
					padding: "8px",
					borderRadius: "5px",
					border: "1px solid rgba(0,0,0,0.1)",
				}}
			>
				<option value="">Toutes régions</option>
				<option value="fr">France</option>
				<option value="us">États-Unis</option>
				<option value="uk">Royaume-Uni</option>
			</select>

			<select
				value={language}
				onChange={(e) => setLanguage(e.target.value)}
				style={{
					padding: "8px",
					borderRadius: "5px",
					border: "1px solid rgba(0,0,0,0.1)",
				}}
			>
				<option value="">Toutes langues</option>
				<option value="fr">Français</option>
				<option value="en">Anglais</option>
			</select>

			<div style={{ display: "flex", gap: "0.5rem" }}>
				<button
					type="button"
					disabled={isFetching}
					onClick={() => handlePagination("prev")}
					style={{
						flex: 1,
						padding: "8px",
						borderRadius: "5px",
						border: "1px solid rgba(0,0,0,0.1)",
						backgroundColor: "white",
						cursor: "pointer",
						opacity: isFetching ? 0.5 : 1,
					}}
				>
					Prev
				</button>
				<button
					type="button"
					disabled={isFetching}
					onClick={() => handlePagination("next")}
					style={{
						flex: 1,
						padding: "8px",
						borderRadius: "5px",
						border: "1px solid rgba(0,0,0,0.1)",
						backgroundColor: "white",
						cursor: "pointer",
						opacity: isFetching ? 0.5 : 1,
					}}
				>
					Next
				</button>
				<button
					type="button"
					disabled={isFetching}
					onClick={() => handlePagination("first")}
					style={{
						padding: "8px",
						borderRadius: "5px",
						border: "1px solid rgba(0,0,0,0.1)",
						backgroundColor: "white",
						cursor: "pointer",
						opacity: isFetching ? 0.5 : 1,
					}}
				>
					First
				</button>
			</div>
		</div>
	);
}

function App() {
	const {
		page,
		search,
		dateRange,
		region,
		language,
		setPage,
		setSearch,
		setDateRange,
		setRegion,
		setLanguage,
	} = useUrl();

	const { data, isFetching, error, isError } = useSearchQuery(
		search,
		page,
		dateRange,
		region,
		language,
	);

	const [showSuggestions, setShowSuggestions] = useState(false);
	const [lastScrollY, setLastScrollY] = useState(0);
	const inputRef = useRef<HTMLInputElement>(null);
	const searchBarRef = useRef<HTMLDivElement>(null);
	const [showConfig, setShowConfig] = useState(false);
	const configButtonRef = useRef<HTMLButtonElement>(null);
	const popoverRef = useRef<HTMLDivElement>(null);
	const suggestionsRef = useRef<HTMLDivElement>(null);

	const { data: suggestions = [] } = useAutocompleteQuery(
		search,
		showSuggestions,
	);
	const { data: quickAnswers = [] } = useQuickAnswersQuery(search);

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
		setSearch(value);
		setShowSuggestions(true);
		if (value.length === 0) {
			setPage(1);
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

	useEffect(() => {
		const handleClickOutside = (event: MouseEvent) => {
			if (
				showConfig &&
				popoverRef.current &&
				!popoverRef.current.contains(event.target as Node) &&
				!configButtonRef.current?.contains(event.target as Node)
			) {
				setShowConfig(false);
			}
		};

		const handleEscape = (event: KeyboardEventInit) => {
			if (event.key === "Escape") {
				setShowConfig(false);
			}
		};

		document.addEventListener("mousedown", handleClickOutside);
		document.addEventListener("keydown", handleEscape);
		return () => {
			document.removeEventListener("mousedown", handleClickOutside);
			document.removeEventListener("keydown", handleEscape);
		};
	}, [showConfig]);

	useEffect(() => {
		const handleClickOutside = (event: MouseEvent) => {
			if (
				!inputRef.current?.contains(event.target as Node) &&
				!suggestionsRef.current?.contains(event.target as Node)
			) {
				setShowSuggestions(false);
			}
		};

		document.addEventListener("mousedown", handleClickOutside);
		return () => document.removeEventListener("mousedown", handleClickOutside);
	}, []);

	useEffect(() => {
		const handleKeyDown = (event: globalThis.KeyboardEvent) => {
			const focusedResult = document.activeElement?.tagName === "A";
			if (
				focusedResult &&
				(event.key === "ArrowDown" || event.key === "ArrowUp")
			) {
				event.preventDefault();
			}
		};

		document.addEventListener("keydown", handleKeyDown);
		return () => document.removeEventListener("keydown", handleKeyDown);
	}, []);

	const setSelectedSuggestion = (suggestion: string) => {
		const cleanSuggestion = suggestion.replace(/<[^>]*>?/gm, "");
		const cleanSearch = search.replace(/<[^>]*>?/gm, "");
		return cleanSuggestion === cleanSearch;
	};

	const setSuggestionTabIndex = (suggestion: string) => {
		const cleanSuggestion = suggestion.replace(/<[^>]*>?/gm, "");
		const cleanSearch = search.replace(/<[^>]*>?/gm, "");
		return cleanSuggestion === cleanSearch ? 0 : -1;
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
					transition: "transform 0.3s",
				}}
				ref={searchBarRef}
			>
				<div
					style={{
						display: "flex",
						gap: "1rem",
						alignItems: "center",
						width: "100%",
						position: "relative",
					}}
				>
					<div style={{ position: "relative", flex: 1 }}>
						<input
							ref={inputRef}
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
									setShowSuggestions(false);
									event.currentTarget.blur();
								}
								if (event.key === "Escape") {
									event.preventDefault();
									setShowSuggestions(false);
								}
								if (event.key === "ArrowDown") {
									event.preventDefault();
									if (showSuggestions) {
										const suggestionsElement =
											document.getElementById("suggestions");
										if (suggestionsElement) {
											const firstSuggestion = suggestionsElement.querySelector(
												".suggestion",
											) as HTMLDivElement | null;
											if (firstSuggestion) {
												firstSuggestion.focus();
											}
										}
									} else {
										const firstResult = document.querySelector("a");
										console.log(firstResult);
										if (firstResult) {
											firstResult.focus();
											firstResult.scrollIntoView({
												behavior: "smooth",
												block: "center",
												inline: "center",
											});
										}
									}
								}
							}}
							style={{
								padding: "8px 16px",
								borderRadius: "10px",
								border: "1px solid rgba(0,0,0,0.1)",
								width: "100%",
								backgroundColor: "rgba(255,255,255,0.9)",
								backdropFilter: "blur(40px)",
							}}
						/>
						{showSuggestions && (
							<Suspense fallback={<div>Loading suggestions...</div>}>
								{suggestions.length > 0 && (
									<div
										aria-labelledby="search"
										aria-activedescendant="suggestions"
										aria-expanded={showSuggestions}
										tabIndex={0}
										ref={suggestionsRef}
										id="suggestions"
										// biome-ignore lint/a11y/useSemanticElements: <explanation>
										role="listbox"
										style={{
											position: "absolute",
											borderRadius: "10px",
											boxShadow: "0 4px 6px rgba(0,0,0,0.1)",
											margin: "4px 0",
											zIndex: 2,
											display: "flex",
											flexDirection: "column",
											width: "100%",
											backgroundColor: "white",
											listStyle: "none",
											overflow: "hidden",
										}}
									>
										{suggestions.map((suggestion, index) => (
											<div
												className="suggestion"
												role="option"
												aria-selected={setSelectedSuggestion(suggestion)}
												tabIndex={setSuggestionTabIndex(suggestion)}
												// biome-ignore lint/security/noDangerouslySetInnerHtml: <explanation>
												dangerouslySetInnerHTML={{ __html: suggestion }}
												key={suggestion}
												onClick={() => {
													const cleanSuggestion = suggestion.replace(
														/<[^>]*>?/gm,
														"",
													);
													setSearch(cleanSuggestion);
													setShowSuggestions(false);
												}}
												onKeyDown={(event) => {
													if (event.key === "Enter") {
														const cleanSuggestion = suggestion.replace(
															/<[^>]*>?/gm,
															"",
														);
														setSearch(cleanSuggestion);
														setShowSuggestions(false);
													} else if (
														event.key === "ArrowDown" &&
														index < suggestions.length - 1
													) {
														event.preventDefault();
														const nextSibling = event.currentTarget
															.nextElementSibling as HTMLLIElement;
														const suggestions =
															document.querySelectorAll(".suggestion");
														for (const suggestion of suggestions) {
															suggestion.setAttribute("aria-selected", "false");
															suggestion.setAttribute("tabIndex", "-1");
														}
														nextSibling?.setAttribute("aria-selected", "true");
														nextSibling?.setAttribute("tabIndex", "0");
														nextSibling?.focus();
													} else if (event.key === "ArrowUp") {
														event.preventDefault();
														if (index === 0) {
															inputRef.current?.focus();
														} else {
															const prevSibling = event.currentTarget
																.previousElementSibling as HTMLLIElement;
															prevSibling?.focus();
															const suggestions =
																document.querySelectorAll(".suggestion");
															for (const suggestion of suggestions) {
																suggestion.setAttribute(
																	"aria-selected",
																	"false",
																);
																suggestion.setAttribute("tabIndex", "-1");
															}
															prevSibling?.setAttribute(
																"aria-selected",
																"true",
															);
															prevSibling?.setAttribute("tabIndex", "0");
														}
													} else if (event.key === "Escape") {
														event.preventDefault();
														inputRef.current?.focus();
														setShowSuggestions(false);
													}
												}}
												style={{
													padding: "8px 16px",
													cursor: "pointer",
													backgroundColor: "white",
												}}
											/>
										))}
									</div>
								)}
							</Suspense>
						)}
					</div>

					<button
						ref={configButtonRef}
						type="button"
						onClick={() => setShowConfig(!showConfig)}
						style={{
							padding: "8px",
							borderRadius: "5px",
							border: "1px solid rgba(0,0,0,0.1)",
							backgroundColor: "white",
							cursor: "pointer",
							display: "flex",
							alignItems: "center",
							justifyContent: "center",
						}}
					>
						⚙️
					</button>

					{showConfig && (
						<PopoverConfig
							dateRange={dateRange}
							setDateRange={setDateRange}
							region={region}
							setRegion={setRegion}
							language={language}
							setLanguage={setLanguage}
							page={page}
							isFetching={isFetching}
							handlePagination={handlePagination}
							popoverRef={popoverRef}
						/>
					)}
				</div>
			</div>

			<div>
				<Suspense fallback={<div>Loading answers...</div>}>
					{quickAnswers.length > 0 && (
						<div style={{ maxWidth: "100%", width: "100%" }}>
							{quickAnswers.map((quickAnswer) => (
								<div
									key={
										quickAnswer.term +
										quickAnswer.definition +
										quickAnswer.source
									}
									style={{
										padding: "1rem",
										backgroundColor: "rgba(255,255,255,0.8)",
										borderRadius: "10px",
										border: "1px solid rgba(0,0,0,0.1)",
										width: "100%",
										maxWidth: "100%",
										marginBlock: "1rem",
										wordBreak: "break-word",
									}}
								>
									<h3 style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
										{quickAnswer.term}
									</h3>
									<div
										style={{
											overflow: "hidden",
											wordWrap: "break-word",
										}}
										// biome-ignore lint/security/noDangerouslySetInnerHtml: <explanation>
										dangerouslySetInnerHTML={{ __html: quickAnswer.definition }}
									/>
									{quickAnswer.source && (
										<p
											style={{
												fontSize: "0.8rem",
												marginTop: "0.5rem",
												overflow: "hidden",
												textOverflow: "ellipsis",
											}}
										>
											Source: {quickAnswer.source}
										</p>
									)}
								</div>
							))}
						</div>
					)}
				</Suspense>
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
									onKeyDown={(event) => {
										if (event.key === "Escape") {
											event.currentTarget.blur();
										}
										if (event.key === "ArrowDown") {
											const nextLiA =
												event.currentTarget.parentElement?.nextElementSibling?.querySelector(
													"a",
												);
											nextLiA?.focus();
										}
										if (event.key === "ArrowUp") {
											const prevLiA =
												event.currentTarget.parentElement?.previousElementSibling?.querySelector(
													"a",
												);
											if (prevLiA) {
												prevLiA.focus();
											} else {
												inputRef.current?.focus();
											}
										}
									}}
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
													}}
												>
													{item.site_name && (
														<p
															style={{
																fontSize: "0.9rem",
															}}
														>
															{item.site_name}
														</p>
													)}
													{item.breadcrumbs && item.breadcrumbs.length > 0 && (
														<ul
															style={{
																display: "flex",
																gap: "0.15rem",
																listStyleType: "none",
																fontSize: "0.75rem",
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
												fontSize: "1.35rem",
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
										i
									</p>
								</a>
							</li>
						))}
					</ul>
				)}
			</div>
			<PWABadge />
		</div>
	);
}

export default App;
