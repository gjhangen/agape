import React, { useState, useEffect, useRef } from 'react';
import { Search, ShoppingCart, MapPin, Clock, Star, User, Home, Calendar, Ticket, Heart, ChevronRight, X, Info, ArrowRight, CreditCard, Package, Check, ArrowLeft, MessageSquare } from 'lucide-react';

const AgapeInteractiveDemo = () => {
  // Navigation & Core State
  const [activeTab, setActiveTab] = useState('home');
  const [showStickyNav, setShowStickyNav] = useState(false);
  const [currentSection, setCurrentSection] = useState('Fan Favorites');
  const mainContentRef = useRef(null);
  const categoriesScrollRef = useRef(null);
  const vendorsScrollRef = useRef(null);
  
  // Cart State
  const [cart, setCart] = useState([]);
  const [isCartOpen, setIsCartOpen] = useState(false);
  const [orderPlaced, setOrderPlaced] = useState(false);
  const [orderStatus, setOrderStatus] = useState(null);
  const [deliveryOption, setDeliveryOption] = useState('seat');
  const [paymentMethod, setPaymentMethod] = useState(null);
  const [couponCode, setCouponCode] = useState('');
  const [couponApplied, setCouponApplied] = useState(false);
  const [couponDiscount, setCouponDiscount] = useState(0);
  const cartRef = useRef(null);
  
  // Search State
  const [searchQuery, setSearchQuery] = useState('');
  const [searchFocused, setSearchFocused] = useState(false);
  const [searchSuggestions] = useState([
    'Hot Dog', 'Beer', 'Nachos', 'Pizza', 'Soda', 'Burger'
  ]);
  
  // Menu Interaction State
  const [showAllItems, setShowAllItems] = useState(false);
  const [showAllEvents, setShowAllEvents] = useState(false);
  const [showAllVendors, setShowAllVendors] = useState(false);
  const [showAllCategories, setShowAllCategories] = useState(false);
  const [activeCategory, setActiveCategory] = useState('All');
  const [addedItemId, setAddedItemId] = useState(null);
  const [favorites, setFavorites] = useState([]);
  const [showReviews, setShowReviews] = useState(false);
  const [selectedItemForReview, setSelectedItemForReview] = useState(null);
  
  // Modals & Popups
  const [showSeatSelector, setShowSeatSelector] = useState(false);
  const [showPremiumInfo, setShowPremiumInfo] = useState(false);
  const [showTicketSelector, setShowTicketSelector] = useState(false);
  const [selectedEvent, setSelectedEvent] = useState(null);
  const [showVenueEvents, setShowVenueEvents] = useState(false);
  const [selectedVenue, setSelectedVenue] = useState(null);
  const [seatInfo, setSeatInfo] = useState(null);
  const [showSection, setShowSection] = useState(null);
  
  // Form Values
  const [sectionInput, setSectionInput] = useState('');
  const [rowInput, setRowInput] = useState('');
  const [seatInput, setSeatInput] = useState('');
  
  // Game info with countdown
  const [gameInfo, setGameInfo] = useState({
    homeTeam: "Knicks",
    awayTeam: "Celtics",
    homeScore: 58,
    awayScore: 52,
    quarter: "2nd",
    timeRemaining: "4:35",
    possession: "home"
  });
  
  // Sample data
  const categories = [
    { id: "All", name: "All", icon: "🍽️" },
    { id: "Hot Dogs", name: "Hot Dogs", icon: "🌭" },
    { id: "Burgers", name: "Burgers", icon: "🍔" },
    { id: "Pizza", name: "Pizza", icon: "🍕" },
    { id: "Nachos", name: "Nachos", icon: "🧀" },
    { id: "Drinks", name: "Drinks", icon: "🥤" },
    { id: "Desserts", name: "Desserts", icon: "🍦" },
    { id: "Snacks", name: "Snacks", icon: "🍿" },
    { id: "Merchandise", name: "Merchandise", icon: "👕" }
  ];
  
  const venues = [
    { id: 1, name: "Madison Square Garden", location: "New York, NY", image: "/api/placeholder/400/200" },
    { id: 2, name: "Staples Center", location: "Los Angeles, CA", image: "/api/placeholder/400/200" },
    { id: 3, name: "Wrigley Field", location: "Chicago, IL", image: "/api/placeholder/400/200" }
  ];
  
  const menuItems = [
    // Original items
    { id: 1, name: "Classic Hot Dog", price: 7.99, vendor: "Stadium Classics", rating: 4.5, reviews: 128, category: "Hot Dogs", image: "/api/placeholder/120/120", description: "Classic all-beef hot dog on a fresh bun" },
    { id: 2, name: "Nachos Supreme", price: 9.99, vendor: "Nacho Nation", rating: 4.7, reviews: 256, category: "Nachos", image: "/api/placeholder/120/120", description: "Crispy tortilla chips with cheese, jalapeños, and salsa" },
    { id: 3, name: "Craft Beer 16oz", price: 12.99, vendor: "Brew Bros", rating: 4.8, reviews: 312, category: "Drinks", image: "/api/placeholder/120/120", description: "Local craft IPA, cold and refreshing" },
    { id: 4, name: "Loaded Fries", price: 8.99, vendor: "Fry Factory", rating: 4.6, reviews: 189, category: "Snacks", image: "/api/placeholder/120/120", description: "Crispy fries topped with cheese, bacon bits, and sour cream" },
    { id: 5, name: "Cheeseburger", price: 10.99, vendor: "Burger Boss", rating: 4.4, reviews: 148, category: "Burgers", image: "/api/placeholder/120/120", description: "⅓ pound Angus beef with cheese and special sauce" },
    { id: 6, name: "Pepperoni Pizza Slice", price: 6.99, vendor: "Pizza Plaza", rating: 4.3, reviews: 167, category: "Pizza", image: "/api/placeholder/120/120", description: "NY-style thin crust with pepperoni" },
    { id: 7, name: "Soft Pretzel", price: 5.99, vendor: "Twisted", rating: 4.2, reviews: 94, category: "Snacks", image: "/api/placeholder/120/120", description: "Warm, soft pretzel with salt and mustard dip" },
    { id: 8, name: "Chocolate Shake", price: 7.99, vendor: "Shake Shack", rating: 4.7, reviews: 203, category: "Desserts", image: "/api/placeholder/120/120", description: "Thick, creamy chocolate shake with whipped cream" },
    
    // Additional items
    { id: 9, name: "Veggie Burger", price: 11.99, vendor: "Burger Boss", rating: 4.3, reviews: 112, category: "Burgers", image: "/api/placeholder/120/120", description: "Plant-based patty with fresh toppings" },
    { id: 10, name: "Italian Sausage", price: 8.99, vendor: "Stadium Classics", rating: 4.6, reviews: 132, category: "Hot Dogs", image: "/api/placeholder/120/120", description: "Grilled Italian sausage with peppers and onions" },
    { id: 11, name: "Loaded Nachos", price: 12.99, vendor: "Nacho Nation", rating: 4.5, reviews: 178, category: "Nachos", image: "/api/placeholder/120/120", description: "Nachos loaded with beef, cheese, jalapeños, and sour cream" },
    { id: 12, name: "Cheese Pizza Slice", price: 5.99, vendor: "Pizza Plaza", rating: 4.4, reviews: 143, category: "Pizza", image: "/api/placeholder/120/120", description: "Classic cheese pizza slice" },
    { id: 13, name: "Premium Hot Dog", price: 9.99, vendor: "Stadium Classics", rating: 4.7, reviews: 145, category: "Hot Dogs", image: "/api/placeholder/120/120", description: "Premium hot dog with choice of gourmet toppings" },
    { id: 14, name: "Garlic Parmesan Fries", price: 7.99, vendor: "Fry Factory", rating: 4.8, reviews: 165, category: "Snacks", image: "/api/placeholder/120/120", description: "Crispy fries tossed with garlic, parmesan, and herbs" },
    { id: 15, name: "Cotton Candy", price: 4.99, vendor: "Sweet Treats", rating: 4.3, reviews: 89, category: "Desserts", image: "/api/placeholder/120/120", description: "Fluffy, sweet cotton candy in team colors" },
    { id: 16, name: "Premium Lager", price: 13.99, vendor: "Brew Bros", rating: 4.5, reviews: 203, category: "Drinks", image: "/api/placeholder/120/120", description: "Imported premium lager, served cold" },
    { id: 17, name: "Team Jersey", price: 89.99, vendor: "Team Store", rating: 4.6, reviews: 215, category: "Merchandise", image: "/api/placeholder/120/120", description: "Official team jersey, home colors" },
    { id: 18, name: "Team Cap", price: 29.99, vendor: "Team Store", rating: 4.5, reviews: 119, category: "Merchandise", image: "/api/placeholder/120/120", description: "Official team cap with embroidered logo" }
  ];

  // Recommended add-ons (upsells)
  const upsellItems = [
    { id: 101, name: "Dipping Sauce", price: 0.99, image: "/api/placeholder/50/50", description: "Choice of ranch, BBQ, or honey mustard" },
    { id: 102, name: "Cheese Topping", price: 1.49, image: "/api/placeholder/50/50", description: "Add melted cheese to any item" },
    { id: 103, name: "Side of Fries", price: 3.99, image: "/api/placeholder/50/50", description: "Small side of crispy fries" }
  ];

  // Group menu items by category
  const menuByCategory = {};
  categories.forEach(category => {
    if (category.id !== 'All') {
      menuByCategory[category.id] = menuItems.filter(item => item.category === category.id);
    }
  });

  const vendors = [
    { id: 1, name: "Stadium Classics", rating: 4.5, image: "/api/placeholder/80/80" },
    { id: 2, name: "Nacho Nation", rating: 4.7, image: "/api/placeholder/80/80" },
    { id: 3, name: "Brew Bros", rating: 4.8, image: "/api/placeholder/80/80" },
    { id: 4, name: "Fry Factory", rating: 4.6, image: "/api/placeholder/80/80" },
    { id: 5, name: "Burger Boss", rating: 4.4, image: "/api/placeholder/80/80" }
  ];
  
  const upcomingEvents = [
    { id: 1, name: "Lakers vs. Warriors", date: "Mar 6, 2025", venue: "Staples Center", image: "/api/placeholder/150/100", ticketsAvailable: true, lowestPrice: 89 },
    { id: 2, name: "Taylor Swift Concert", date: "Mar 10, 2025", venue: "Madison Square Garden", image: "/api/placeholder/150/100", ticketsAvailable: true, lowestPrice: 199 },
    { id: 3, name: "Cubs vs. Cardinals", date: "Mar 12, 2025", venue: "Wrigley Field", image: "/api/placeholder/150/100", ticketsAvailable: true, lowestPrice: 65 },
    { id: 4, name: "Bruno Mars Concert", date: "Mar 15, 2025", venue: "Madison Square Garden", image: "/api/placeholder/150/100", ticketsAvailable: true, lowestPrice: 149 }
  ];

  const venueEvents = {
    "Madison Square Garden": upcomingEvents.filter(event => event.venue === "Madison Square Garden"),
    "Staples Center": upcomingEvents.filter(event => event.venue === "Staples Center"),
    "Wrigley Field": upcomingEvents.filter(event => event.venue === "Wrigley Field")
  };

  const generateSeats = (event) => {
    const sections = ["100", "101", "102", "200", "201", "202"];
    const seats = [];
    
    sections.forEach(section => {
      const basePrice = event.lowestPrice;
      const sectionPrice = section.startsWith("1") ? basePrice * 1.5 : basePrice * 1.2;
      
      seats.push({
        section,
        price: Math.round(sectionPrice),
        available: Math.floor(Math.random() * 30) + 5
      });
    });
    
    return seats;
  };
  
  // Game Score Timer
  useEffect(() => {
    const timer = setInterval(() => {
      setGameInfo(prev => {
        const timeArr = prev.timeRemaining.split(':');
        let minutes = parseInt(timeArr[0]);
        let seconds = parseInt(timeArr[1]);
        
        seconds -= 1;
        if (seconds < 0) {
          seconds = 59;
          minutes -= 1;
        }
        
        if (minutes < 0) {
          clearInterval(timer);
          return prev;
        } else {
          return {
            ...prev,
            timeRemaining: `${minutes}:${seconds < 10 ? '0' + seconds : seconds}`
          };
        }
      });
    }, 3000); // Slowed down to 3 seconds for demo
    
    return () => clearInterval(timer);
  }, []);
  
  // Cart Animation Effect
  useEffect(() => {
    if (addedItemId !== null) {
      const timer = setTimeout(() => {
        setAddedItemId(null);
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, [addedItemId]);

  // Order status update
  useEffect(() => {
    if (orderPlaced && orderStatus) {
      const statuses = ["preparing", "ready", "delivering", "delivered"];
      const currentIdx = statuses.indexOf(orderStatus.status);
      
      if (currentIdx < statuses.length - 1) {
        const timer = setTimeout(() => {
          setOrderStatus({
            ...orderStatus,
            status: statuses[currentIdx + 1],
            timeRemaining: orderStatus.timeRemaining > 0 ? orderStatus.timeRemaining - 2 : 0
          });
        }, 4000);
        
        return () => clearTimeout(timer);
      }
    }
  }, [orderPlaced, orderStatus]);

  // Sticky nav scroll handling
  useEffect(() => {
    const handleScroll = () => {
      if (mainContentRef.current) {
        const scrollPosition = mainContentRef.current.scrollTop;
        setShowStickyNav(scrollPosition > 250);
        
        // Determine current section
        const sections = mainContentRef.current.querySelectorAll('[data-section]');
        
        for (let i = sections.length - 1; i >= 0; i--) {
          const section = sections[i];
          const rect = section.getBoundingClientRect();
          
          if (rect.top <= 150) {
            setCurrentSection(section.dataset.section);
            break;
          }
        }
      }
    };
    
    const contentRef = mainContentRef.current;
    if (contentRef) {
      contentRef.addEventListener('scroll', handleScroll);
      return () => contentRef.removeEventListener('scroll', handleScroll);
    }
  }, []);
  
  // Search functionality
  const handleSearch = (e) => {
    setSearchQuery(e.target.value);
  };

  const handleSelectSearchSuggestion = (suggestion) => {
    setSearchQuery(suggestion);
    setSearchFocused(false);
  };
  
  const filteredMenuItems = menuItems.filter(item => {
    const matchesCategory = activeCategory === 'All' || item.category === activeCategory;
    const matchesSearch = searchQuery === '' || 
      item.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      item.vendor.toLowerCase().includes(searchQuery.toLowerCase()) ||
      item.category.toLowerCase().includes(searchQuery.toLowerCase());
    
    return matchesCategory && matchesSearch;
  });
  
  const addToCart = (item) => {
    setCart([...cart, item]);
    setAddedItemId(item.id);
    // Open cart briefly to show item was added
    setIsCartOpen(true);
    setTimeout(() => {
      setIsCartOpen(false);
    }, 1200);
  };
  
  const removeFromCart = (index) => {
    const newCart = [...cart];
    newCart.splice(index, 1);
    setCart(newCart);
  };
  
  // Full-screen category view modal
  const AllCategoriesModal = () => {
    if (!showAllCategories) return null;
    
    return (
      <div className="fixed inset-0 z-50 bg-white flex flex-col">
        <div className="p-4 border-b border-gray-200 flex items-center">
          <button 
            onClick={() => setShowAllCategories(false)}
            className="p-2 -ml-2 mr-2 rounded-full hover:bg-gray-100"
          >
            <ArrowLeft size={24} className="text-gray-700" />
          </button>
          <h2 className="text-xl font-bold flex-grow">All Categories</h2>
        </div>
        
        <div className="p-4 overflow-y-auto flex-grow">
          <div className="grid grid-cols-2 gap-4">
            {categories.map(category => (
              <button 
                key={category.id}
                onClick={() => {
                  setActiveCategory(category.id);
                  setShowAllCategories(false);
                }}
                className={`px-4 py-8 rounded-lg shadow-sm flex flex-col items-center justify-center ${
                  activeCategory === category.id ? 'bg-red-100 border-2 border-red-500' : 'bg-white border border-gray-200'
                }`}
              >
                <span className="text-3xl mb-2">{category.icon}</span>
                <span className={`font-medium ${activeCategory === category.id ? 'text-red-700' : 'text-gray-800'}`}>
                  {category.name}
                </span>
              </button>
            ))}
          </div>
        </div>
      </div>
    );
  };
  
  // Full-screen vendors view modal
  const AllVendorsModal = () => {
    if (!showAllVendors) return null;
    
    return (
      <div className="fixed inset-0 z-50 bg-white flex flex-col">
        <div className="p-4 border-b border-gray-200 flex items-center">
          <button 
            onClick={() => setShowAllVendors(false)}
            className="p-2 -ml-2 mr-2 rounded-full hover:bg-gray-100"
          >
            <ArrowLeft size={24} className="text-gray-700" />
          </button>
          <h2 className="text-xl font-bold flex-grow">All Vendors</h2>
        </div>
        
        <div className="p-4 overflow-y-auto flex-grow">
          <div className="grid grid-cols-2 gap-4">
            {vendors.map(vendor => (
              <div key={vendor.id} className="bg-white rounded-lg border border-gray-200 p-4 flex flex-col items-center">
                <img src={vendor.image} alt={vendor.name} className="w-20 h-20 rounded-full mb-3" />
                <div className="text-center">
                  <div className="font-medium">{vendor.name}</div>
                  <div className="flex items-center justify-center text-sm text-gray-500 mt-1">
                    <Star size={16} className="text-yellow-500 mr-1" fill="currentColor" /> 
                    {vendor.rating}
                  </div>
                </div>
                <button className="mt-3 text-red-600 border border-red-600 rounded-full px-4 py-1 text-sm">
                  View Menu
                </button>
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  };
  
  // Full-screen items view modal
  const AllItemsModal = () => {
    if (!showAllItems) return null;
    
    return (
      <div className="fixed inset-0 z-50 bg-white flex flex-col">
        <div className="p-4 border-b border-gray-200 flex items-center">
          <button 
            onClick={() => setShowAllItems(false)}
            className="p-2 -ml-2 mr-2 rounded-full hover:bg-gray-100"
          >
            <ArrowLeft size={24} className="text-gray-700" />
          </button>
          <h2 className="text-xl font-bold flex-grow">Fan Favorites</h2>
        </div>
        
        <div className="p-4 overflow-y-auto flex-grow">
          <div className="grid grid-cols-2 gap-4">
            {filteredMenuItems.map(item => (
              <div key={item.id} className="bg-white rounded-lg shadow-sm overflow-hidden border border-gray-200">
                <div className="relative">
                  <img src={item.image} alt={item.name} className="w-full h-32 object-cover" />
                  <button 
                    onClick={() => toggleFavorite(item.id)}
                    className="absolute top-2 right-2 bg-white rounded-full p-1 shadow"
                  >
                    <Heart 
                      size={18} 
                      className={favorites.includes(item.id) ? "text-red-500" : "text-gray-400"} 
                      fill={favorites.includes(item.id) ? "currentColor" : "none"} 
                    />
                  </button>
                </div>
                <div className="p-3">
                  <div className="font-medium">{item.name}</div>
                  <div className="text-sm text-gray-500">{item.vendor}</div>
                  <div 
                    className="flex items-center mt-1 text-sm cursor-pointer"
                    onClick={() => openReviews(item)}
                  >
                    <Star size={14} className="text-yellow-500" fill="currentColor" />
                    <span className="font-medium ml-1">{item.rating}</span>
                    <span className="text-blue-500 ml-1 underline flex items-center">
                      ({item.reviews})
                      <MessageSquare size={12} className="ml-1" />
                    </span>
                  </div>
                  <div className="flex justify-between items-center mt-2">
                    <div className="font-bold">${item.price}</div>
                    <button 
                      onClick={() => {
                        addToCart(item);
                        setShowAllItems(false);
                      }}
                      className="bg-red-600 text-white p-1 rounded-full w-8 h-8 flex items-center justify-center text-lg"
                    >
                      +
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  };
  
  // Full-screen events view modal
  const AllEventsModal = () => {
    if (!showAllEvents) return null;
    
    return (
      <div className="fixed inset-0 z-50 bg-white flex flex-col">
        <div className="p-4 border-b border-gray-200 flex items-center">
          <button 
            onClick={() => setShowAllEvents(false)}
            className="p-2 -ml-2 mr-2 rounded-full hover:bg-gray-100"
          >
            <ArrowLeft size={24} className="text-gray-700" />
          </button>
          <h2 className="text-xl font-bold flex-grow">All Upcoming Events</h2>
        </div>
        
        <div className="p-4 overflow-y-auto flex-grow">
          <div className="space-y-4">
            {upcomingEvents.map(event => (
              <div key={event.id} className="bg-white rounded-lg shadow-sm overflow-hidden border border-gray-200">
                <img src={event.image} alt={event.name} className="w-full h-40 object-cover" />
                <div className="p-4">
                  <h3 className="font-bold text-lg">{event.name}</h3>
                  <div 
                    className="flex items-center text-gray-500 mb-2 cursor-pointer"
                    onClick={() => {
                      setShowAllEvents(false);
                      openVenueEvents(event.venue);
                    }}
                  >
                    <MapPin size={16} className="mr-1" />
                    <span className="underline">{event.venue}</span>
                  </div>
                  <div className="flex items-center text-gray-500 mb-3">
                    <Calendar size={16} className="mr-1" />
                    {event.date}
                  </div>
                  <div className="flex justify-between items-center">
                    <div>
                      <div className="text-xs text-gray-500">Starting at</div>
                      <div className="font-bold text-red-600">${event.lowestPrice}</div>
                    </div>
                    <button 
                      className="bg-red-600 text-white py-2 px-4 rounded-full text-sm font-bold flex items-center"
                      onClick={() => {
                        setShowAllEvents(false);
                        openTicketSelector(event);
                      }}
                    >
                      <Ticket size={14} className="mr-1" /> Buy Tickets
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  };

  const openTicketSelector = (event) => {
    setSelectedEvent(event);
    setShowTicketSelector(true);
  };

  const handleSeatSubmit = () => {
    if (sectionInput && rowInput && seatInput) {
      setSeatInfo({
        section: sectionInput,
        row: rowInput,
        seat: seatInput
      });
      setShowSeatSelector(false);
      setSearchFocused(true);
    }
  };

  const toggleFavorite = (itemId) => {
    if (favorites.includes(itemId)) {
      setFavorites(favorites.filter(id => id !== itemId));
    } else {
      setFavorites([...favorites, itemId]);
    }
  };

  const openReviews = (item) => {
    setSelectedItemForReview(item);
    setShowReviews(true);
  };

  // Generate sample reviews for menu items
  const getReviewsForItem = (itemId) => {
    // Generate 5-12 random reviews for each item
    const reviewCount = Math.floor(Math.random() * 8) + 5;
    const reviews = [];
    
    const reviewTexts = [
      "Great food, would definitely order again!",
      "Pretty good, but a bit expensive for what you get.",
      "Exactly what I expected. Solid stadium food.",
      "The best I've had at any venue. Worth every penny!",
      "Decent, but not amazing. Gets the job done.",
      "Loved it! My go-to every time I come to a game.",
      "Fresh and delicious. Quick delivery too!",
      "The quality has improved since last time.",
      "Better than most stadium food, but not restaurant quality.",
      "Perfect game day treat!",
      "Portion size was bigger than expected.",
      "Tastes homemade, which is impressive for stadium food.",
      "A bit salty for my taste, but overall good.",
      "Arrived hot and fresh. Would recommend.",
      "Always consistent quality.",
      "Pretty standard, nothing special but satisfying."
    ];
    
    const names = [
      "John S.", "Maria L.", "David W.", "Sarah T.", "Michael B.", 
      "Emily R.", "James K.", "Jessica M.", "Robert P.", "Jennifer D.",
      "Thomas C.", "Lisa H.", "Daniel F.", "Michelle G.", "William J.",
      "Patricia A.", "Richard N.", "Linda S.", "Charles T.", "Barbara M."
    ];
    
    for (let i = 0; i < reviewCount; i++) {
      // Generate random rating, but weighted toward the item's average rating
      const baseRating = menuItems.find(item => item.id === itemId)?.rating || 4.5;
      // Generate a rating within 1 point of the base rating
      let rating = baseRating + (Math.random() * 2 - 1);
      // Make sure rating is between 1 and 5
      rating = Math.max(1, Math.min(5, rating));
      // Round to nearest 0.5
      rating = Math.round(rating * 2) / 2;
      
      // Random review text
      const reviewText = reviewTexts[Math.floor(Math.random() * reviewTexts.length)];
      
      // Random name
      const name = names[Math.floor(Math.random() * names.length)];
      
      // Random date within last 3 months
      const date = new Date();
      date.setDate(date.getDate() - Math.floor(Math.random() * 90));
      const formattedDate = `${date.toLocaleString('default', { month: 'short' })} ${date.getDate()}, ${date.getFullYear()}`;
      
      reviews.push({
        id: i + 1,
        rating,
        text: reviewText,
        user: name,
        date: formattedDate
      });
    }
    
    // Sort reviews by date (newest first)
    return reviews.sort((a, b) => new Date(b.date) - new Date(a.date));
  };
  // Prepare displayed lists
  const displayedItems = filteredMenuItems.slice(0, 4);
  const displayedVendors = vendors.slice(0, 5);

  // Render the sticky navigation
  const StickyNavigation = () => {
    if (!showStickyNav) return null;
    
    return (
      <div className="fixed top-0 left-0 right-0 bg-white shadow-md z-40 max-w-md mx-auto">
        <div className="px-4 py-2">
          <div className="font-medium text-gray-800">Full Menu</div>
          <div className="flex overflow-x-auto space-x-2 mt-1">
            {categories.filter(cat => cat.id !== 'All').map(category => (
              <button
                key={category.id}
                onClick={() => {
                  // Find the section and scroll to it
                  const section = document.querySelector(`[data-section="${category.id}"]`);
                  if (section) {
                    section.scrollIntoView({ behavior: 'smooth' });
                  }
                }}
                className={`px-3 py-1 rounded-full text-xs whitespace-nowrap flex items-center ${
                  currentSection === category.id
                    ? 'bg-red-600 text-white'
                    : 'bg-gray-100 text-gray-800'
                }`}
              >
                <span className="mr-1">{category.icon}</span>
                {category.name}
              </button>
            ))}
          </div>
        </div>
      </div>
    );
  };

  const SeatSelectorModal = () => {
    if (!showSeatSelector) return null;
    
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
        <div className="bg-white rounded-lg max-w-sm w-full p-4">
          <h2 className="text-xl font-bold mb-4">Where are you sitting?</h2>
          <p className="text-gray-600 text-sm mb-4">We'll deliver your order right to your seat.</p>
          
          <div className="space-y-3 mb-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Section</label>
              <input 
                type="text" 
                className="w-full p-2 border border-gray-300 rounded"
                placeholder="e.g. 214"
                value={sectionInput}
                onChange={(e) => setSectionInput(e.target.value)}
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Row</label>
              <input 
                type="text" 
                className="w-full p-2 border border-gray-300 rounded"
                placeholder="e.g. F"
                value={rowInput}
                onChange={(e) => setRowInput(e.target.value)}
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">Seat</label>
              <input 
                type="text" 
                className="w-full p-2 border border-gray-300 rounded"
                placeholder="e.g. 12"
                value={seatInput}
                onChange={(e) => setSeatInput(e.target.value)}
              />
            </div>
          </div>
          
          <div className="flex space-x-2">
            <button 
              onClick={() => setShowSeatSelector(false)}
              className="flex-1 py-2 border border-gray-300 rounded-full"
            >
              Cancel
            </button>
            <button 
              onClick={handleSeatSubmit}
              className="flex-1 py-2 bg-red-600 text-white rounded-full"
              disabled={!sectionInput || !rowInput || !seatInput}
            >
              Confirm
            </button>
          </div>
        </div>
      </div>
    );
  };

  const PremiumInfoModal = () => {
    if (!showPremiumInfo) return null;
    
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
        <div className="bg-white rounded-lg max-w-sm w-full p-4">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-xl font-bold">Agape Premium</h2>
            <button onClick={() => setShowPremiumInfo(false)}>
              <X size={24} />
            </button>
          </div>
          
          <div className="mb-4">
            <div className="bg-blue-100 text-blue-800 px-4 py-3 rounded-lg mb-4">
              <p className="font-bold text-lg">$5.99/month or $59.99/year</p>
            </div>
            
            <h3 className="font-medium mb-2">Membership includes:</h3>
            <ul className="space-y-2 mb-4">
              <li className="flex items-start">
                <Check size={16} className="text-green-500 mr-2 mt-0.5" />
                <span>$0 delivery fees on eligible orders</span>
              </li>
              <li className="flex items-start">
                <Check size={16} className="text-green-500 mr-2 mt-0.5" />
                <span>5% off all food and drink purchases</span>
              </li>
              <li className="flex items-start">
                <Check size={16} className="text-green-500 mr-2 mt-0.5" />
                <span>Early access to event tickets</span>
              </li>
              <li className="flex items-start">
                <Check size={16} className="text-green-500 mr-2 mt-0.5" />
                <span>Exclusive premium menu items</span>
              </li>
            </ul>
          </div>
          
          <button 
            className="w-full py-3 bg-blue-600 text-white rounded-full font-bold"
            onClick={() => setShowPremiumInfo(false)}
          >
            Join Agape Premium
          </button>
          <button 
            className="w-full py-2 text-blue-600 mt-2"
            onClick={() => setShowPremiumInfo(false)}
          >
            No thanks
          </button>
        </div>
      </div>
    );
  };

  const TicketSelectorModal = () => {
    if (!showTicketSelector || !selectedEvent) return null;
    
    const seats = generateSeats(selectedEvent);
    
    return (
      <div className="fixed inset-0 z-50 bg-white flex flex-col" style={{ transition: 'opacity 0.3s ease' }}>
        <div className="p-4 border-b border-gray-200 flex items-center">
          <button 
            onClick={() => setShowTicketSelector(false)}
            className="p-2 -ml-2 mr-2 rounded-full hover:bg-gray-100"
          >
            <ArrowLeft size={24} className="text-gray-700" />
          </button>
          <h2 className="text-xl font-bold flex-grow">{selectedEvent.name} - Select Tickets</h2>
        </div>
        
        <div className="p-4">
          <div className="bg-white rounded-lg mb-4">
            <div className="flex items-center mb-4">
              <img 
                src={selectedEvent.image} 
                alt={selectedEvent.name} 
                className="w-24 h-24 object-cover rounded mr-3"
              />
              <div>
                <h3 className="font-medium text-lg">{selectedEvent.name}</h3>
                <p className="text-gray-600">{selectedEvent.venue} • {selectedEvent.date}</p>
              </div>
            </div>
            
            {/* Venue map */}
            <div className="mb-6">
              <h3 className="font-medium mb-2">Venue Map</h3>
              <div className="relative bg-gray-100 p-2 rounded-lg overflow-hidden">
                <div className="w-full aspect-[3/2] bg-slate-200 rounded-lg relative overflow-hidden">
                  {/* Stylized venue map */}
                  <div className="absolute inset-0 flex items-center justify-center">
                    <div className="w-3/4 h-1/2 bg-slate-300 rounded-full flex items-center justify-center">
                      <div className="w-1/2 h-28 bg-white rounded-lg flex items-center justify-center text-sm font-medium text-gray-700">
                        {selectedEvent.venue === "Madison Square Garden" ? "Court" : "Stage"}
                      </div>
                    </div>
                  </div>
                  
                  {/* Section highlights */}
                  {seats.map((seat, index) => {
                    // Calculate position based on section number
                    const angle = (parseInt(seat.section.replace(/\D/g, '')) % 100) / 100 * 360;
                    const radius = seat.section.startsWith('1') ? 85 : 
                                 seat.section.startsWith('2') ? 70 : 55;
                    const x = 50 + Math.cos(angle * Math.PI / 180) * radius / 2;
                    const y = 50 + Math.sin(angle * Math.PI / 180) * radius / 2;
                    
                    return (
                      <div 
                        key={seat.section}
                        className={`absolute w-12 h-6 flex items-center justify-center text-xs font-medium rounded-full transition-all transform -translate-x-1/2 -translate-y-1/2 cursor-pointer ${
                          showSection === seat.section ? 'bg-red-500 text-white' : 'bg-white text-gray-800'
                        } hover:bg-red-100 border border-gray-200`}
                        style={{
                          left: `${x}%`,
                          top: `${y}%`,
                        }}
                        onClick={() => setShowSection(seat.section)}
                      >
                        {seat.section}
                      </div>
                    );
                  })}
                </div>
                <p className="text-xs text-gray-500 text-center mt-2">
                  Click on a section to see available seats
                </p>
              </div>
            </div>
            
            <h3 className="font-medium mb-2">Available Sections</h3>
            <div className="space-y-3 max-h-80 overflow-y-auto">
              {seats.map((seat, index) => (
                <div 
                  key={index} 
                  className={`p-3 border rounded-lg flex justify-between items-center cursor-pointer transition ${
                    showSection === seat.section 
                      ? 'border-red-500 bg-red-50'
                      : 'border-gray-200 bg-white hover:bg-gray-50'
                  }`}
                  onClick={() => setShowSection(seat.section)}
                >
                  <div>
                    <div className="font-medium">Section {seat.section}</div>
                    <div className="text-sm text-gray-500">{seat.available} seats available</div>
                  </div>
                  <div className="text-right">
                    <div className="font-bold">${seat.price}</div>
                    <div className="text-xs text-gray-500">per ticket</div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
        
        <div className="mt-auto p-4 border-t border-gray-200">
          <button 
            className={`w-full py-3 rounded-full font-bold ${
              showSection ? 'bg-red-600 text-white' : 'bg-gray-300 text-gray-500'
            }`}
            disabled={!showSection}
            onClick={() => {
              alert(`You selected Section ${showSection}. This would continue to seat selection.`);
              setShowTicketSelector(false);
            }}
          >
            {showSection ? `Continue with Section ${showSection}` : 'Select a section'}
          </button>
        </div>
      </div>
    );
  };

  const ReviewsModal = () => {
    if (!showReviews || !selectedItemForReview) return null;
    
    const reviews = getReviewsForItem(selectedItemForReview.id);
    
    // Calculate ratings distribution
    const ratingCounts = { 5: 0, 4: 0, 3: 0, 2: 0, 1: 0 };
    reviews.forEach(review => {
      const roundedRating = Math.floor(review.rating);
      ratingCounts[roundedRating] = (ratingCounts[roundedRating] || 0) + 1;
    });
    
    const totalReviews = reviews.length;
    
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
        <div className="bg-white rounded-lg max-w-sm w-full p-4 max-h-[80vh] overflow-y-auto">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-xl font-bold">Reviews</h2>
            <button onClick={() => setShowReviews(false)}>
              <X size={24} />
            </button>
          </div>
          
          <div className="mb-4">
            <div className="flex items-center mb-2">
              <img 
                src={selectedItemForReview.image} 
                alt={selectedItemForReview.name} 
                className="w-16 h-16 object-cover rounded mr-3" 
              />
              <div>
                <h3 className="font-medium">{selectedItemForReview.name}</h3>
                <div className="text-sm text-gray-500">{selectedItemForReview.vendor}</div>
                <div className="flex items-center mt-1">
                  <Star size={16} className="text-yellow-500" fill="currentColor" />
                  <span className="mx-1 font-medium">{selectedItemForReview.rating}</span>
                  <span className="text-gray-500">({selectedItemForReview.reviews} reviews)</span>
                </div>
              </div>
            </div>
            
            {/* Ratings distribution */}
            <div className="bg-gray-50 p-3 rounded-lg mb-4">
              <h4 className="font-medium text-sm mb-2">Ratings</h4>
              {[5, 4, 3, 2, 1].map(rating => (
                <div key={rating} className="flex items-center mb-1 last:mb-0">
                  <div className="w-8 text-sm">{rating} ★</div>
                  <div className="flex-grow mx-2 bg-gray-200 h-2 rounded-full overflow-hidden">
                    <div 
                      className="bg-yellow-500 h-full rounded-full"
                      style={{ width: `${(ratingCounts[rating] / totalReviews) * 100}%` }}
                    ></div>
                  </div>
                  <div className="text-xs text-gray-500 w-8 text-right">{ratingCounts[rating] || 0}</div>
                </div>
              ))}
            </div>
          </div>
          
          <div className="space-y-4">
            {reviews.map(review => (
              <div key={review.id} className="border-b border-gray-100 pb-4 last:border-b-0">
                <div className="flex justify-between items-start mb-1">
                  <div className="font-medium">{review.user}</div>
                  <div className="text-xs text-gray-500">{review.date}</div>
                </div>
                <div className="flex mb-2">
                  {[1, 2, 3, 4, 5].map(star => (
                    <Star 
                      key={star} 
                      size={14} 
                      className={star <= review.rating ? "text-yellow-500" : "text-gray-300"} 
                      fill={star <= review.rating ? "currentColor" : "none"} 
                    />
                  ))}
                </div>
                <p className="text-sm text-gray-700">{review.text}</p>
              </div>
            ))}
          </div>
          
          <button 
            className="w-full py-3 bg-red-600 text-white rounded-full font-bold mt-4"
            onClick={() => setShowReviews(false)}
          >
            Close
          </button>
        </div>
      </div>
    );
  };

  const VenueEventsModal = () => {
    if (!showVenueEvents || !selectedVenue) return null;
    
    const events = venueEvents[selectedVenue] || [];
    
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
        <div className="bg-white rounded-lg max-w-sm w-full p-4 max-h-[80vh] overflow-y-auto">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-xl font-bold">Events at {selectedVenue}</h2>
            <button onClick={() => setShowVenueEvents(false)}>
              <X size={24} />
            </button>
          </div>
          
          <div className="space-y-4 mb-4">
            {events.length > 0 ? (
              events.map(event => (
                <div 
                  key={event.id} 
                  className="border border-gray-200 rounded-lg overflow-hidden"
                >
                  <img src={event.image} alt={event.name} className="w-full h-32 object-cover" />
                  <div className="p-3">
                    <h3 className="font-medium">{event.name}</h3>
                    <div className="flex items-center text-sm text-gray-500 mb-2">
                      <Calendar size={14} className="mr-1" /> {event.date}
                    </div>
                    <div className="flex justify-between items-center">
                      <div className="font-bold text-red-600">From ${event.lowestPrice}</div>
                      <button 
                        className="bg-red-600 text-white py-1 px-3 rounded-full text-sm"
                        onClick={() => {
                          setShowVenueEvents(false);
                          openTicketSelector(event);
                        }}
                      >
                        Buy Tickets
                      </button>
                    </div>
                  </div>
                </div>
              ))
            ) : (
              <p className="text-center text-gray-500 py-4">No upcoming events at this venue</p>
            )}
          </div>
          
          <button 
            className="w-full py-3 bg-red-600 text-white rounded-full font-bold"
            onClick={() => setShowVenueEvents(false)}
          >
            Close
          </button>
        </div>
      </div>
    );
  };

  // Render the Home screen
  const renderHomeScreen = () => (
    <div className="p-4 pb-20 overflow-y-auto" ref={mainContentRef}>
      {/* Current Game Banner with Score */}
      <div className="bg-red-600 text-white p-4 rounded-lg mb-6">
        <div className="font-bold text-lg">Now at Madison Square Garden</div>
        <div className="flex justify-between items-center my-2">
          <div className="text-center">
            <div className="font-bold">{gameInfo.homeTeam}</div>
            <div className="text-2xl font-bold">{gameInfo.homeScore}</div>
          </div>
          <div className="text-center opacity-80">
            <div>{gameInfo.quarter} Quarter</div>
            <div className="font-bold">{gameInfo.timeRemaining}</div>
          </div>
          <div className="text-center">
            <div className="font-bold">{gameInfo.awayTeam}</div>
            <div className="text-2xl font-bold">{gameInfo.awayScore}</div>
          </div>
        </div>
        <div className="flex space-x-2 mt-2">
          <button 
            onClick={() => setShowSeatSelector(true)}
            className="flex-1 bg-white text-red-600 py-1 px-3 rounded-full text-sm font-bold"
          >
            Order to your seat
          </button>
          <button 
            onClick={() => setSearchFocused(true)}
            className="flex-1 bg-red-600 text-white border border-white py-1 px-3 rounded-full text-sm font-bold"
          >
            Order for pickup
          </button>
        </div>
      </div>
      
      {/* Search Bar */}
      <div className="relative mb-6">
        <input 
          type="text" 
          value={searchQuery}
          onChange={handleSearch}
          onFocus={() => setSearchFocused(true)}
          placeholder="Search for food, drinks, or vendors" 
          className="w-full py-3 pl-10 pr-4 bg-gray-100 rounded-full"
        />
        <Search className="absolute left-3 top-3 text-gray-400" size={20} />
      </div>
      
      {/* Categories with navigation arrows */}
      <div className="mb-6 relative">
        <div className="flex justify-between items-center mb-2">
          <h2 className="font-bold text-lg">Categories</h2>
        </div>
        <div className="relative mb-1">
          <div 
            id="categoriesScroll"
            className="flex overflow-x-scroll overflow-y-hidden space-x-2 pb-2" 
            ref={categoriesScrollRef}
            style={{ 
              scrollbarWidth: 'none', /* Firefox */
              msOverflowStyle: 'none', /* IE and Edge */
              WebkitOverflowScrolling: 'touch'
            }}
          >
            <style>
              {`
                #categoriesScroll::-webkit-scrollbar {
                  display: none;
                }
              `}
            </style>
            {categories.map((category, index) => {
              const isActive = activeCategory === category.id;
              return (
                <button
                  key={category.id}
                  id={`category-${category.id}`}
                  onClick={() => {
                    setActiveCategory(category.id);
                    // Center the selected category
                    const element = document.getElementById(`category-${category.id}`);
                    if (element && categoriesScrollRef.current) {
                      const container = categoriesScrollRef.current;
                      const scrollLeft = element.offsetLeft - (container.clientWidth / 2) + (element.clientWidth / 2);
                      container.scrollTo({ left: scrollLeft, behavior: 'smooth' });
                    }
                  }}
                  className={`px-4 py-2 rounded-full text-sm whitespace-nowrap flex items-center ${
                    isActive ? 'bg-red-600 text-white' : 'bg-gray-100 text-gray-800'
                  } flex-shrink-0`}
                >
                  <span className="mr-2 text-lg">{category.icon}</span>
                  {category.name}
                </button>
              );
            })}
          </div>
        </div>
        <div className="flex justify-center space-x-4 mt-1">
          <button 
            className="w-8 h-8 rounded-full bg-white shadow-md flex items-center justify-center text-gray-600 hover:bg-gray-100"
            onClick={() => {
              if (categoriesScrollRef.current) {
                categoriesScrollRef.current.scrollBy({ left: -100, behavior: 'smooth' });
              }
            }}
          >
            <ChevronRight className="transform rotate-180" size={20} />
          </button>
          
          <button 
            className="w-8 h-8 rounded-full bg-white shadow-md flex items-center justify-center text-gray-600 hover:bg-gray-100"
            onClick={() => {
              if (categoriesScrollRef.current) {
                categoriesScrollRef.current.scrollBy({ left: 100, behavior: 'smooth' });
              }
            }}
          >
            <ChevronRight size={20} />
          </button>
        </div>
      </div>
      
      {/* Fan Favorites */}
      <div className="mb-8" data-section="Fan Favorites">
        <div className="flex justify-between items-center mb-3">
          <h2 className="font-bold text-lg">Fan Favorites</h2>
          <button 
            onClick={() => setShowAllItems(true)} 
            className="text-red-600 text-sm flex items-center"
          >
            See All
            <ChevronRight size={16} />
          </button>
        </div>
        <div className="grid grid-cols-2 gap-4">
          {displayedItems.slice(0, 4).map(item => (
            <div key={item.id} className="bg-white rounded-lg shadow-sm overflow-hidden">
              <div className="relative">
                <img src={item.image} alt={item.name} className="w-full h-32 object-cover" />
                <button 
                  onClick={() => toggleFavorite(item.id)}
                  className="absolute top-2 right-2 bg-white rounded-full p-1 shadow"
                >
                  <Heart 
                    size={18} 
                    className={favorites.includes(item.id) ? "text-red-500" : "text-gray-400"} 
                    fill={favorites.includes(item.id) ? "currentColor" : "none"} 
                  />
                </button>
              </div>
              <div className="p-3">
                <div className="font-medium">{item.name}</div>
                <div className="text-sm text-gray-500">{item.vendor}</div>
                <div 
                  className="flex items-center mt-1 text-sm cursor-pointer"
                  onClick={() => openReviews(item)}
                >
                  <Star size={14} className="text-yellow-500" fill="currentColor" />
                  <span className="font-medium ml-1">{item.rating}</span>
                  <span className="text-blue-500 ml-1 underline flex items-center">
                    ({item.reviews})
                    <MessageSquare size={12} className="ml-1" />
                  </span>
                </div>
                <div className="flex justify-between items-center mt-2">
                  <div className="font-bold">${item.price}</div>
                  <button 
                    onClick={() => addToCart(item)}
                    className={`${
                      addedItemId === item.id 
                        ? 'bg-green-500' 
                        : 'bg-red-600'
                    } text-white p-1 rounded-full w-8 h-8 flex items-center justify-center text-lg transition-colors duration-300`}
                  >
                    {addedItemId === item.id ? '✓' : '+'}
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
      
      {/* Vendors */}
      <div className="mb-8" data-section="Vendors">
        <div className="flex justify-between items-center mb-2">
          <h2 className="font-bold text-lg">Vendors</h2>
          <button 
            onClick={() => setShowAllVendors(true)} 
            className="text-red-600 text-sm flex items-center"
          >
            See All
            <ChevronRight size={16} />
          </button>
        </div>
        <div className="relative mb-1">
          <div 
            id="vendorsScroll"
            className="flex overflow-x-scroll space-x-3 pb-2" 
            ref={vendorsScrollRef}
            style={{ 
              scrollbarWidth: 'none', /* Firefox */
              msOverflowStyle: 'none', /* IE and Edge */
              WebkitOverflowScrolling: 'touch'
            }}
          >
            <style>
              {`
                #vendorsScroll::-webkit-scrollbar {
                  display: none;
                }
              `}
            </style>
            {displayedVendors.map((vendor, index) => (
              <div key={vendor.id} className="flex-shrink-0 w-28 bg-white rounded-lg shadow-sm p-3">
                <img src={vendor.image} alt={vendor.name} className="w-16 h-16 rounded-full mx-auto mb-2" />
                <div className="text-center">
                  <div className="font-medium text-sm truncate">{vendor.name}</div>
                  <div className="flex items-center justify-center text-xs text-gray-500 mt-1">
                    <Star size={12} className="text-yellow-500 mr-1" fill="currentColor" /> 
                    {vendor.rating}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
        
        <div className="flex justify-center space-x-4 mt-1">
          <button 
            className="w-8 h-8 rounded-full bg-white shadow-md flex items-center justify-center text-gray-600 hover:bg-gray-100"
            onClick={() => {
              if (vendorsScrollRef.current) {
                vendorsScrollRef.current.scrollBy({ left: -100, behavior: 'smooth' });
              }
            }}
          >
            <ChevronRight className="transform rotate-180" size={20} />
          </button>
          
          <button 
            className="w-8 h-8 rounded-full bg-white shadow-md flex items-center justify-center text-gray-600 hover:bg-gray-100"
            onClick={() => {
              if (vendorsScrollRef.current) {
                vendorsScrollRef.current.scrollBy({ left: 100, behavior: 'smooth' });
              }
            }}
          >
            <ChevronRight size={20} />
          </button>
        </div>
      </div>
      
      {/* Full Menu */}
      <div className="mb-8" data-section="Full Menu">
        <div className="flex justify-between items-center mb-3">
          <h2 className="font-bold text-lg">Full Menu</h2>
        </div>
        <div className="grid grid-cols-1 gap-3">
          {categories.filter(cat => cat.id !== "All").map(category => (
            <div key={category.id} className="mb-4" data-section={category.id}>
              <h3 className="font-medium text-gray-700 mb-2 flex items-center">
                <span className="mr-2 text-xl">{category.icon}</span>
                {category.name}
              </h3>
              <div className="grid grid-cols-1 gap-3">
                {(menuByCategory[category.id] || []).slice(0, showAllItems ? undefined : 3).map(item => (
                  <div key={item.id} className="flex items-center bg-white p-3 rounded-lg shadow-sm">
                    <img src={item.image} alt={item.name} className="w-16 h-16 object-cover rounded mr-3" />
                    <div className="flex-grow">
                      <div className="font-medium">{item.name}</div>
                      <div className="text-sm text-gray-500">{item.vendor}</div>
                      <div className="flex items-center mt-1 text-sm">
                        <Star size={14} className="text-yellow-500" fill="currentColor" />
                        <span className="font-medium ml-1">{item.rating}</span>
                        <span 
                          className="text-blue-500 ml-1 underline flex items-center cursor-pointer"
                          onClick={(e) => {
                            e.stopPropagation();
                            openReviews(item);
                          }}
                        >
                          ({item.reviews})
                          <MessageSquare size={12} className="ml-1" />
                        </span>
                      </div>
                      <div className="flex justify-between items-center mt-1">
                        <div className="font-bold">${item.price}</div>
                        <div className="flex items-center">
                          <button 
                            onClick={() => toggleFavorite(item.id)}
                            className="mr-2"
                          >
                            <Heart 
                              size={18} 
                              className={favorites.includes(item.id) ? "text-red-500" : "text-gray-400"} 
                              fill={favorites.includes(item.id) ? "currentColor" : "none"} 
                            />
                          </button>
                          <button 
                            onClick={() => addToCart(item)}
                            className="bg-red-600 text-white p-1 rounded-full w-6 h-6 flex items-center justify-center text-sm"
                          >
                            +
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
                {!showAllItems && (menuByCategory[category.id] || []).length > 3 && (
                  <button 
                    onClick={() => setShowAllItems(true)}
                    className="text-red-600 text-sm font-medium flex items-center justify-center py-2"
                  >
                    See all {category.name} <ChevronRight size={16} />
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
      
      {/* Upcoming Events */}
      <div className="mb-8" data-section="Events">
        <div className="flex justify-between items-center mb-3">
          <h2 className="font-bold text-lg">Upcoming Events</h2>
          <button 
            onClick={() => setShowAllEvents(true)} 
            className="text-red-600 text-sm flex items-center"
          >
            See All
            <ChevronRight size={16} />
          </button>
        </div>
        <div className="grid grid-cols-1 gap-4">
          {upcomingEvents.slice(0, 2).map(event => (
            <div key={event.id} className="bg-white rounded-lg shadow-sm overflow-hidden">
              <div className="flex">
                <img src={event.image} alt={event.name} className="w-24 h-24 object-cover" />
                <div className="p-3 flex-grow">
                  <div className="font-medium">{event.name}</div>
                  <div 
                    className="text-sm text-gray-500 underline cursor-pointer" 
                    onClick={() => openVenueEvents(event.venue)}
                  >
                    {event.venue}
                  </div>
                  <div className="text-sm text-gray-500 mt-1 flex items-center">
                    <Calendar size={14} className="mr-1" /> {event.date}
                  </div>
                  <div className="mt-2 flex justify-between items-center">
                    {event.ticketsAvailable ? (
                      <div className="text-sm">
                        <span className="font-bold text-red-600">From ${event.lowestPrice}</span>
                      </div>
                    ) : (
                      <div className="text-sm text-gray-500">Sold Out</div>
                    )}
                    <button 
                      onClick={() => openTicketSelector(event)}
                      className="bg-red-600 text-white py-1 px-3 rounded-full text-xs font-bold flex items-center"
                    >
                      <Ticket size={12} className="mr-1" /> Tickets
                    </button>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
  
  // Render the Search View
  const renderSearchView = () => (
    <div 
      className="fixed inset-0 bg-white z-40 p-4 max-w-md mx-auto search-animation"
      style={{
        animation: 'search-slide-up 0.3s ease-out'
      }}
    >
      <style>
        {`
          @keyframes search-slide-up {
            from {
              opacity: 0;
              transform: translateY(20px);
            }
            to {
              opacity: 1;
              transform: translateY(0);
            }
          }
        `}
      </style>
      <div className="flex items-center mb-6">
        <button 
          onClick={() => setSearchFocused(false)}
          className="p-2 -ml-2 mr-2"
        >
          <ArrowLeft size={24} className="text-gray-700" />
        </button>
        <div className="relative flex-grow">
          <input 
            type="text" 
            value={searchQuery}
            onChange={handleSearch}
            autoFocus
            placeholder="What do you want to eat?" 
            className="w-full py-3 pl-10 pr-4 bg-gray-100 rounded-full"
          />
          <Search className="absolute left-3 top-3 text-gray-400" size={20} />
          {searchQuery && (
            <button 
              onClick={() => setSearchQuery('')}
              className="absolute right-3 top-3"
            >
              <X size={20} className="text-gray-400" />
            </button>
          )}
        </div>
      </div>
      
      {searchQuery ? (
        <div>
          <h2 className="font-medium mb-2">Results for "{searchQuery}"</h2>
          <div className="space-y-3">
            {filteredMenuItems.length > 0 ? (
              filteredMenuItems.map(item => (
                <div key={item.id} className="flex items-center bg-white p-3 rounded-lg shadow-sm">
                  <img src={item.image} alt={item.name} className="w-16 h-16 object-cover rounded mr-3" />
                  <div className="flex-grow">
                    <div className="font-medium">{item.name}</div>
                    <div className="text-sm text-gray-500">{item.vendor}</div>
                    <div className="flex justify-between items-center mt-1">
                      <div className="font-bold">${item.price}</div>
                      <button 
                        onClick={() => {
                          addToCart(item);
                          setSearchFocused(false);
                        }}
                        className="bg-red-600 text-white p-1 rounded-full w-6 h-6 flex items-center justify-center text-sm"
                      >
                        +
                      </button>
                    </div>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-8 text-gray-500">
                No results found for "{searchQuery}"
              </div>
            )}
          </div>
        </div>
      ) : (
        <div>
          <h2 className="font-medium mb-3">Popular Searches</h2>
          <div className="flex flex-wrap gap-2 mb-6">
            {searchSuggestions.map(suggestion => (
              <button 
                key={suggestion}
                onClick={() => handleSelectSearchSuggestion(suggestion)}
                className="px-4 py-2 bg-gray-100 rounded-full text-sm"
              >
                {suggestion}
              </button>
            ))}
          </div>
          
          <div className="grid grid-cols-1 gap-3 mb-6">
            <h2 className="font-medium">Categories</h2>
            {categories.filter(cat => cat.id !== 'All').map(category => (
              <button 
                key={category.id}
                onClick={() => {
                  setActiveCategory(category.id);
                  setSearchFocused(false);
                }}
                className="flex items-center justify-between p-3 bg-white rounded-lg shadow-sm"
              >
                <div className="flex items-center">
                  <span className="mr-3 text-xl">{category.icon}</span>
                  <span>{category.name}</span>
                </div>
                <ChevronRight size={20} className="text-gray-400" />
              </button>
            ))}
          </div>
          
          <button 
            onClick={() => setSearchFocused(false)}
            className="w-full py-3 border border-red-600 text-red-600 rounded-full font-medium"
          >
            Browse entire venue
          </button>
        </div>
      )}
    </div>
  );
  
  // Render the Explore screen
  const renderExploreScreen = () => (
    <div className="p-4 pb-20 overflow-y-auto">
      <h1 className="text-2xl font-bold mb-4">Events & Tickets</h1>
      
      {/* Categories */}
      <div className="mb-6">
        <div className="flex overflow-x-auto space-x-2 pb-2">
          {["All Events", "Sports", "Concerts", "Theater", "Family"].map(category => (
            <button
              key={category}
              className={`px-4 py-2 rounded-full text-sm whitespace-nowrap ${
                category === "All Events"
                  ? 'bg-red-600 text-white'
                  : 'bg-gray-100 text-gray-800'
              }`}
            >
              {category}
            </button>
          ))}
        </div>
      </div>
      
      {/* Upcoming Events */}
      <div className="mb-8">
        <h2 className="font-bold text-lg mb-3">Upcoming Events</h2>
        <div className="space-y-4">
          {upcomingEvents.map(event => (
            <div key={event.id} className="bg-white rounded-lg shadow overflow-hidden">
              <img src={event.image} alt={event.name} className="w-full h-40 object-cover" />
              <div className="p-4">
                <h3 className="font-bold text-lg">{event.name}</h3>
                <div 
                  className="flex items-center text-gray-500 mb-2 cursor-pointer"
                  onClick={() => openVenueEvents(event.venue)}
                >
                  <MapPin size={16} className="mr-1" />
                  <span className="underline">{event.venue}</span>
                </div>
                <div className="flex items-center text-gray-500 mb-3">
                  <Calendar size={16} className="mr-1" />
                  {event.date}
                </div>
                <div className="flex justify-between items-center">
                  <div>
                    <div className="text-xs text-gray-500">Starting at</div>
                    <div className="font-bold text-red-600">${event.lowestPrice}</div>
                  </div>
                  <button 
                    className="bg-red-600 text-white py-2 px-4 rounded-full text-sm font-bold flex items-center"
                    onClick={() => openTicketSelector(event)}
                  >
                    <Ticket size={14} className="mr-1" /> Buy Tickets
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
      
      {/* Venues */}
      <div>
        <h2 className="font-bold text-lg mb-3">Popular Venues</h2>
        <div className="space-y-4">
          {venues.map(venue => (
            <div key={venue.id} className="bg-white rounded-lg shadow overflow-hidden">
              <img src={venue.image} alt={venue.name} className="w-full h-32 object-cover" />
              <div className="p-4">
                <h3 className="font-bold text-lg">{venue.name}</h3>
                <div className="flex items-center text-gray-500">
                  <MapPin size={16} className="mr-1" />
                  {venue.location}
                </div>
                <button 
                  className="mt-3 border border-red-600 text-red-600 py-1 px-3 rounded-full text-sm font-medium"
                  onClick={() => openVenueEvents(venue.name)}
                >
                  View Events
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
  
  // Render the Profile screen
  const renderProfileScreen = () => (
    <div className="p-4 pb-20 overflow-y-auto">
      <h1 className="text-2xl font-bold mb-4">Profile</h1>
      
      <div className="bg-white rounded-lg shadow-sm p-4 mb-4 flex items-center">
        <div className="bg-red-600 w-16 h-16 rounded-full flex items-center justify-center text-white text-2xl font-bold mr-4">
          JS
        </div>
        <div>
          <div className="font-bold text-lg">John Smith</div>
          <div className="text-gray-500">john.smith@example.com</div>
        </div>
      </div>
      
      <div className="bg-white rounded-lg shadow-sm overflow-hidden mb-4">
        <button 
          className="w-full p-4 border-b border-gray-100 flex items-center"
          onClick={() => setShowSection('order-history')}
        >
          <div className="bg-red-100 p-2 rounded mr-3">
            <Clock className="text-red-600" size={20} />
          </div>
          <div className="flex-grow">Order History</div>
          <ChevronRight size={18} className="text-gray-400" />
        </button>
        <button 
          className="w-full p-4 border-b border-gray-100 flex items-center"
          onClick={() => setShowSection('my-tickets')}
        >
          <div className="bg-red-100 p-2 rounded mr-3">
            <Ticket className="text-red-600" size={20} />
          </div>
          <div className="flex-grow">My Tickets</div>
          <ChevronRight size={18} className="text-gray-400" />
        </button>
        <button 
          className="w-full p-4 border-b border-gray-100 flex items-center"
          onClick={() => setShowSection('saved-venues')}
        >
          <div className="bg-red-100 p-2 rounded mr-3">
            <MapPin className="text-red-600" size={20} />
          </div>
          <div className="flex-grow">Saved Venues</div>
          <ChevronRight size={18} className="text-gray-400" />
        </button>
        <button 
          className="w-full p-4 border-b border-gray-100 flex items-center"
          onClick={() => setShowSection('favorites')}
        >
          <div className="bg-red-100 p-2 rounded mr-3">
            <Heart className="text-red-600" size={20} />
          </div>
          <div className="flex-grow">Favorites</div>
          <ChevronRight size={18} className="text-gray-400" />
        </button>
      </div>
      
      <button 
        className="w-full border border-red-500 text-red-500 py-3 rounded-full font-bold"
      >
        Sign Out
      </button>
      
      {showSection === 'order-history' && (
        <div className="fixed inset-0 z-50 bg-white p-4 max-w-md mx-auto">
          <div className="flex items-center mb-4">
            <button 
              onClick={() => setShowSection(null)}
              className="p-2 -ml-2 mr-2"
            >
              <ArrowLeft size={24} className="text-gray-700" />
            </button>
            <h2 className="text-xl font-bold">Order History</h2>
          </div>
          
          <div className="space-y-4">
            <div className="bg-white rounded-lg shadow-sm p-4">
              <div className="flex justify-between items-start mb-2">
                <div>
                  <div className="font-medium">Order #AGP7438</div>
                  <div className="text-sm text-gray-500">Mar 1, 2025 • 7:35 PM</div>
                </div>
                <div className="bg-green-100 text-green-800 px-2 py-1 rounded text-xs">Delivered</div>
              </div>
              <div className="border-t border-gray-100 my-2"></div>
              <div className="text-sm">
                <div className="mb-1">1x Classic Hot Dog</div>
                <div className="mb-1">1x Craft Beer 16oz</div>
                <div className="font-medium">Total: $20.98</div>
              </div>
            </div>
            
            <div className="bg-white rounded-lg shadow-sm p-4">
              <div className="flex justify-between items-start mb-2">
                <div>
                  <div className="font-medium">Order #AGP6251</div>
                  <div className="text-sm text-gray-500">Feb 24, 2025 • 6:12 PM</div>
                </div>
                <div className="bg-green-100 text-green-800 px-2 py-1 rounded text-xs">Delivered</div>
              </div>
              <div className="border-t border-gray-100 my-2"></div>
              <div className="text-sm">
                <div className="mb-1">2x Nachos Supreme</div>
                <div className="mb-1">2x Fountain Soda</div>
                <div className="font-medium">Total: $27.96</div>
              </div>
            </div>
          </div>
        </div>
      )}

      {showSection === 'my-tickets' && (
        <div className="fixed inset-0 z-50 bg-white p-4 max-w-md mx-auto">
          <div className="flex items-center mb-4">
            <button 
              onClick={() => setShowSection(null)}
              className="p-2 -ml-2 mr-2"
            >
              <ArrowLeft size={24} className="text-gray-700" />
            </button>
            <h2 className="text-xl font-bold">My Tickets</h2>
          </div>
          
          <div className="space-y-4">
            <div className="bg-white rounded-lg shadow-sm overflow-hidden">
              <div className="bg-red-600 p-4 text-white">
                <div className="font-bold text-lg">Knicks vs. Celtics</div>
                <div className="text-sm opacity-90">Madison Square Garden • Today</div>
              </div>
              <div className="p-4">
                <div className="flex justify-between mb-2">
                  <div className="text-sm text-gray-500">Section</div>
                  <div className="font-medium">214</div>
                </div>
                <div className="flex justify-between mb-2">
                  <div className="text-sm text-gray-500">Row</div>
                  <div className="font-medium">F</div>
                </div>
                <div className="flex justify-between mb-2">
                  <div className="text-sm text-gray-500">Seat</div>
                  <div className="font-medium">12</div>
                </div>
                <div className="flex justify-between mb-2">
                  <div className="text-sm text-gray-500">Ticket Type</div>
                  <div className="font-medium">Standard</div>
                </div>
                
                <div className="border-t border-gray-100 my-3"></div>
                
                <div className="flex justify-center">
                  <img src="/api/placeholder/200/200" alt="QR Code" className="w-36 h-36" />
                </div>
              </div>
            </div>
            
            <div className="bg-white rounded-lg shadow-sm overflow-hidden">
              <div className="bg-blue-600 p-4 text-white">
                <div className="font-bold text-lg">Taylor Swift Concert</div>
                <div className="text-sm opacity-90">Madison Square Garden • Mar 10</div>
              </div>
              <div className="p-4">
                <div className="flex justify-between mb-2">
                  <div className="text-sm text-gray-500">Section</div>
                  <div className="font-medium">105</div>
                </div>
                <div className="flex justify-between mb-2">
                  <div className="text-sm text-gray-500">Row</div>
                  <div className="font-medium">C</div>
                </div>
                <div className="flex justify-between mb-2">
                  <div className="text-sm text-gray-500">Seat</div>
                  <div className="font-medium">7</div>
                </div>
                <div className="flex justify-between mb-2">
                  <div className="text-sm text-gray-500">Ticket Type</div>
                  <div className="font-medium">Premium</div>
                </div>
                
                <div className="border-t border-gray-100 my-3"></div>
                
                <div className="flex justify-center">
                  <img src="/api/placeholder/200/200" alt="QR Code" className="w-36 h-36" />
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {showSection === 'favorites' && (
        <div className="fixed inset-0 z-50 bg-white p-4 max-w-md mx-auto">
          <div className="flex items-center mb-4">
            <button 
              onClick={() => setShowSection(null)}
              className="p-2 -ml-2 mr-2"
            >
              <ArrowLeft size={24} className="text-gray-700" />
            </button>
            <h2 className="text-xl font-bold">Favorites</h2>
          </div>
          
          <div className="space-y-4">
            {favorites.length > 0 ? (
              menuItems.filter(item => favorites.includes(item.id)).map(item => (
                <div key={item.id} className="flex items-center bg-white p-3 rounded-lg shadow-sm">
                  <img src={item.image} alt={item.name} className="w-16 h-16 object-cover rounded mr-3" />
                  <div className="flex-grow">
                    <div className="font-medium">{item.name}</div>
                    <div className="text-sm text-gray-500">{item.vendor}</div>
                    <div className="flex justify-between items-center mt-1">
                      <div className="font-bold">${item.price}</div>
                      <div className="flex items-center">
                        <button 
                          onClick={() => toggleFavorite(item.id)}
                          className="mr-2"
                        >
                          <Heart size={18} className="text-red-500" fill="currentColor" />
                        </button>
                        <button 
                          onClick={() => addToCart(item)}
                          className="bg-red-600 text-white p-1 rounded-full w-6 h-6 flex items-center justify-center text-sm"
                        >
                          +
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-8">
                <Heart size={48} className="mx-auto text-gray-300 mb-3" />
                <h3 className="font-medium text-lg mb-1">No favorites yet</h3>
                <p className="text-gray-500 mb-4">Tap the heart icon to save your favorites</p>
              </div>
            )}
          </div>
        </div>
      )}

      {showSection === 'saved-venues' && (
        <div className="fixed inset-0 z-50 bg-white p-4 max-w-md mx-auto">
          <div className="flex items-center mb-4">
            <button 
              onClick={() => setShowSection(null)}
              className="p-2 -ml-2 mr-2"
            >
              <ArrowLeft size={24} className="text-gray-700" />
            </button>
            <h2 className="text-xl font-bold">Saved Venues</h2>
          </div>
          
          <div className="space-y-4">
            {venues.map(venue => (
              <div key={venue.id} className="bg-white rounded-lg shadow-sm p-4 flex items-center">
                <img src={venue.image} alt={venue.name} className="w-16 h-16 object-cover rounded mr-3" />
                <div className="flex-grow">
                  <div className="font-medium">{venue.name}</div>
                  <div className="text-sm text-gray-500">{venue.location}</div>
                  <button 
                    className="mt-2 text-red-600 text-sm font-medium"
                    onClick={() => openVenueEvents(venue.name)}
                  >
                    View Events
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );

  // Cart component
  const CartDrawer = () => (
    <div 
      ref={cartRef}
      className={`fixed inset-y-0 right-0 w-5/6 max-w-md bg-white shadow-xl transform transition-transform duration-300 ease-in-out z-50 ${
        isCartOpen ? 'translate-x-0' : 'translate-x-full'
      }`}
      style={{ transition: 'transform 0.3s ease-in-out' }}
    >
      <div className="h-full flex flex-col">
        <div className="p-4 border-b border-gray-200 flex items-center">
          <button 
            onClick={() => setIsCartOpen(false)}
            className="p-2 mr-2"
          >
            <X size={24} className="text-gray-700" />
          </button>
          <h2 className="text-xl font-bold flex-grow">Your Cart</h2>
          {cart.length > 0 && (
            <div className="bg-red-100 text-red-600 rounded-full px-2 py-1 text-sm">
              {cart.length} items
            </div>
          )}
        </div>
        
        <div className="flex-grow overflow-y-auto p-4">
          {cart.length === 0 ? (
            <div className="text-center py-8">
              <ShoppingCart size={48} className="mx-auto text-gray-300 mb-3" />
              <h3 className="font-medium text-lg mb-1">Your cart is empty</h3>
              <p className="text-gray-500 mb-4">Add items to start an order</p>
              <button 
                onClick={() => setIsCartOpen(false)}
                className="bg-red-600 text-white py-2 px-4 rounded-full font-medium"
              >
                Browse Menu
              </button>
            </div>
          ) : (
            <div>
              {orderPlaced ? (
                <div className="text-center py-4">
                  <div className="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mx-auto mb-4">
                    <div className="w-8 h-8 text-green-500 text-2xl">✓</div>
                  </div>
                  <h3 className="font-bold text-lg mb-2">Order Placed!</h3>
                  <p className="text-gray-600 mb-1">Your order is on its way</p>
                  <div className="border border-gray-200 rounded-lg p-4 mt-4">
                    <div className="flex justify-between mb-2">
                      <div className="text-sm text-gray-500">Order #</div>
                      <div className="font-medium">{orderStatus?.orderId}</div>
                    </div>
                    <div className="flex justify-between mb-2">
                      <div className="text-sm text-gray-500">Estimated Delivery</div>
                      <div className="font-medium">{orderStatus?.timeRemaining} mins</div>
                    </div>
                    
                    <div className="relative pt-4">
                      <div className="absolute top-0 left-0 w-full h-0.5 bg-gray-200">
                        <div 
                          className="absolute top-0 left-0 h-full bg-green-500"
                          style={{ 
                            width: orderStatus?.status === "preparing" ? "25%" : 
                                   orderStatus?.status === "ready" ? "50%" : 
                                   orderStatus?.status === "delivering" ? "75%" : "100%" 
                          }}
                        ></div>
                      </div>
                      <div className="flex justify-between mt-2">
                        <div className={`text-xs ${orderStatus?.status === "preparing" ? "text-green-500 font-bold" : ""}`}>
                          Preparing
                        </div>
                        <div className={`text-xs ${orderStatus?.status === "ready" ? "text-green-500 font-bold" : ""}`}>
                          Ready
                        </div>
                        <div className={`text-xs ${orderStatus?.status === "delivering" ? "text-green-500 font-bold" : ""}`}>
                          Delivering
                        </div>
                        <div className={`text-xs ${orderStatus?.status === "delivered" ? "text-green-500 font-bold" : ""}`}>
                          Delivered
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              ) : (
                <>
                  {/* Premium Offer */}
                  <div className="bg-blue-50 border border-blue-100 rounded-lg p-3 mb-4 flex items-start">
                    <Info size={20} className="text-blue-500 mr-2 flex-shrink-0 mt-0.5" />
                    <div>
                      <div className="font-medium text-blue-800">Save $5.98 on delivery & service fees with Agape Premium</div>
                      <button 
                        className="text-blue-600 text-sm font-medium mt-1 flex items-center"
                        onClick={() => setShowPremiumInfo(true)}
                      >
                        Learn more <ArrowRight size={14} className="ml-1" />
                      </button>
                    </div>
                  </div>
                    
                  {/* Delivery Options */}
                  <div className="bg-white rounded-lg shadow-sm p-4 mb-4">
                    <h3 className="font-medium mb-3">Delivery Options</h3>
                    <div className="flex mb-3">
                      <button
                        onClick={() => setDeliveryOption('seat')}
                        className={`flex-1 py-2 flex flex-col items-center justify-center rounded-lg transition mr-2 ${
                          deliveryOption === 'seat' ? 'bg-red-50 border-red-500 border-2' : 'border border-gray-200'
                        }`}
                      >
                        <MapPin size={20} className={deliveryOption === 'seat' ? "text-red-500" : "text-gray-500"} />
                        <span className={`text-sm mt-1 ${deliveryOption === 'seat' ? "text-red-500 font-medium" : "text-gray-500"}`}>
                          Seat Delivery
                        </span>
                      </button>
                      <button
                        onClick={() => setDeliveryOption('pickup')}
                        className={`flex-1 py-2 flex flex-col items-center justify-center rounded-lg transition ${
                          deliveryOption === 'pickup' ? 'bg-red-50 border-red-500 border-2' : 'border border-gray-200'
                        }`}
                      >
                        <Package size={20} className={deliveryOption === 'pickup' ? "text-red-500" : "text-gray-500"} />
                        <span className={`text-sm mt-1 ${deliveryOption === 'pickup' ? "text-red-500 font-medium" : "text-gray-500"}`}>
                          Pickup
                        </span>
                      </button>
                    </div>
                    
                    {deliveryOption === 'seat' ? (
                      seatInfo ? (
                        <div className="flex items-center justify-between bg-gray-50 p-3 rounded-lg">
                          <div className="flex items-center">
                            <div className="bg-red-100 text-red-600 p-1 rounded mr-2">
                              <MapPin size={16} />
                            </div>
                            <div>
                              <div className="font-medium">Section {seatInfo.section}, Row {seatInfo.row}, Seat {seatInfo.seat}</div>
                              <div className="text-gray-500 text-sm">Madison Square Garden</div>
                            </div>
                          </div>
                          <button 
                            onClick={() => setShowSeatSelector(true)}
                            className="text-red-600 text-sm"
                          >
                            Edit
                          </button>
                        </div>
                      ) : (
                        <button 
                          onClick={() => setShowSeatSelector(true)}
                          className="w-full py-2 border border-red-600 text-red-600 rounded-lg flex items-center justify-center"
                        >
                          <MapPin size={16} className="mr-1" />
                          Add seat location
                        </button>
                      )
                    ) : (
                      <div className="bg-gray-50 p-3 rounded-lg">
                        <div className="font-medium">Pickup at Main Concourse</div>
                        <div className="text-gray-500 text-sm">Madison Square Garden • Near Gate 4</div>
                      </div>
                    )}
                  </div>
                  
                  {/* Order Details */}
                  <div className="bg-white rounded-lg shadow-sm p-4 mb-4">
                    <div className="font-medium mb-2">Items:</div>
                    {cart.map((item, index) => (
                      <div key={index} className="flex justify-between py-2 border-b border-gray-100 last:border-b-0">
                        <div className="flex">
                          <img src={item.image} alt={item.name} className="w-12 h-12 object-cover rounded mr-3" />
                          <div>
                            <div className="font-medium">{item.name}</div>
                            <div className="text-sm text-gray-500">{item.vendor}</div>
                          </div>
                        </div>
                        <div className="flex items-center">
                          <div className="font-bold mr-2">${item.price}</div>
                          <button 
                            onClick={() => removeFromCart(index)} 
                            className="text-gray-400 hover:text-red-500"
                          >
                            <X size={16} />
                          </button>
                        </div>
                      </div>
                    ))}
                    
                    {/* Recommended add-ons */}
                    {cart.length > 0 && (
                      <div className="mt-4">
                        <h3 className="font-medium mb-2">Recommended Add-ons</h3>
                        <div className="space-y-2">
                          {upsellItems.map(item => (
                            <div key={item.id} className="flex items-center justify-between border border-gray-200 p-2 rounded-lg">
                              <div className="flex items-center">
                                <img src={item.image} alt={item.name} className="w-10 h-10 object-cover rounded mr-2" />
                                <div>
                                  <div className="font-medium text-sm">{item.name}</div>
                                  <div className="text-xs text-gray-500">{item.description}</div>
                                </div>
                              </div>
                              <div className="flex items-center">
                                <div className="text-sm font-bold mr-2">${item.price}</div>
                                <button 
                                  onClick={() => addToCart(item)}
                                  className="bg-red-600 text-white p-0.5 rounded-full w-5 h-5 flex items-center justify-center text-xs"
                                >
                                  +
                                </button>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                    
                    {/* Coupon Code */}
                    <div className="mt-4">
                      <div className="flex mb-2">
                        <input 
                          type="text" 
                          placeholder="Enter coupon code" 
                          className="flex-grow py-2 px-3 border border-gray-300 rounded-l-lg"
                          value={couponCode}
                          onChange={(e) => setCouponCode(e.target.value)}
                        />
                        <button 
                          onClick={applyCoupon}
                          className="bg-red-600 text-white px-3 rounded-r-lg"
                        >
                          Apply
                        </button>
                      </div>
                      
                      {couponApplied && (
                        <div className="text-green-600 text-sm mb-2 flex items-center">
                          <Check size={16} className="mr-1" /> {couponCode.toUpperCase()} applied: ${couponDiscount} off
                        </div>
                      )}
                    </div>
                    
                    <div className="border-t border-gray-100 my-3"></div>
                    
                    {/* Order Summary */}
                    <div>
                      <div className="flex justify-between py-1">
                        <div className="text-gray-500">Subtotal</div>
                        <div>
                          ${cart.reduce((sum, item) => sum + item.price, 0).toFixed(2)}
                        </div>
                      </div>
                      <div className="flex justify-between py-1">
                        <div className="text-gray-500">Delivery Fee</div>
                        <div>{deliveryOption === 'seat' ? '$3.99' : '$0.00'}</div>
                      </div>
                      <div className="flex justify-between py-1">
                        <div className="text-gray-500">Service Fee</div>
                        <div>$1.99</div>
                      </div>
                      {couponApplied && (
                        <div className="flex justify-between py-1">
                          <div className="text-green-600">Discount</div>
                          <div className="text-green-600">-${couponDiscount.toFixed(2)}</div>
                        </div>
                      )}
                      <div className="flex justify-between py-2 font-bold">
                        <div>Total</div>
                        <div>
                          ${(cart.reduce((sum, item) => sum + item.price, 0) + 
                             (deliveryOption === 'seat' ? 3.99 : 0) + 
                             1.99 - 
                             (couponApplied ? couponDiscount : 0)).toFixed(2)}
                        </div>
                      </div>
                    </div>
                  </div>
                  
                  {/* Payment Method */}
                  <div className="bg-white rounded-lg shadow-sm p-4 mb-4">
                    <h3 className="font-medium mb-3">Payment Method</h3>
                    
                    <div className="space-y-2">
                      <button
                        onClick={() => setPaymentMethod('card')}
                        className={`w-full flex items-center justify-between p-3 rounded-lg transition ${
                          paymentMethod === 'card' ? 'bg-red-50 border-red-500 border-2' : 'border border-gray-200'
                        }`}
                      >
                        <div className="flex items-center">
                          <CreditCard size={20} className="mr-2" />
                          <div>
                            <div className={`font-medium ${paymentMethod === 'card' ? "text-red-500" : ""}`}>Credit Card</div>
                            <div className="text-xs text-gray-500">•••• 4587</div>
                          </div>
                        </div>
                        {paymentMethod === 'card' && <Check size={18} className="text-red-500" />}
                      </button>
                      
                      <button
                        onClick={() => setPaymentMethod('apple')}
                        className={`w-full flex items-center justify-between p-3 rounded-lg transition ${
                          paymentMethod === 'apple' ? 'bg-red-50 border-red-500 border-2' : 'border border-gray-200'
                        }`}
                      >
                        <div className="flex items-center">
                          <div className="mr-2 text-lg">🍎</div>
                          <div className={`font-medium ${paymentMethod === 'apple' ? "text-red-500" : ""}`}>Apple Pay</div>
                        </div>
                        {paymentMethod === 'apple' && <Check size={18} className="text-red-500" />}
                      </button>
                      
                      <button className="w-full flex items-center justify-center text-red-600 py-2 text-sm">
                        + Add Payment Method
                      </button>
                    </div>
                  </div>
                </>
              )}
            </div>
          )}
        </div>
        
        {cart.length > 0 && !orderPlaced && (
          <div className="p-4 border-t border-gray-200">
            <button 
              onClick={placeOrder}
              disabled={!paymentMethod || (deliveryOption === 'seat' && !seatInfo)}
              className={`w-full py-3 rounded-full font-bold ${
                paymentMethod && (deliveryOption !== 'seat' || seatInfo)
                  ? 'bg-red-600 text-white'
                  : 'bg-gray-300 text-gray-500'
              }`}
            >
              Place Order
            </button>
          </div>
        )}
        
        {orderStatus && (
          <div className="p-4 border-t border-gray-200">
            <button 
              onClick={() => setIsCartOpen(false)}
              className="w-full bg-red-600 text-white py-3 rounded-full font-bold"
            >
              Close
            </button>
          </div>
        )}
      </div>
    </div>
  );
  
  // Main render function
  const renderScreen = () => {
    switch(activeTab) {
      case 'home':
        return renderHomeScreen();
      case 'explore':
        return renderExploreScreen();
      case 'profile':
        return renderProfileScreen();
      default:
        return renderHomeScreen();
    }
  };
  
  return (
    <div className="max-w-md mx-auto h-screen flex flex-col bg-gray-50 relative border border-gray-200 rounded-xl overflow-hidden">
      {/* Header */}
      <div className="bg-white p-4 shadow-sm flex items-center">
        <div className="flex-grow">
          <h1 className="text-xl font-bold text-red-600">Agape</h1>
          <div className="flex items-center text-sm text-gray-500">
            <MapPin size={14} className="mr-1" />
            Madison Square Garden
          </div>
        </div>
        <div className="flex items-center">
          <button 
            onClick={() => setActiveTab('profile')}
            className="mr-4"
          >
            <User size={24} className="text-gray-700" />
          </button>
          <button 
            onClick={() => setIsCartOpen(!isCartOpen)}
            className="relative"
          >
            <ShoppingCart size={24} className="text-gray-700" />
            {cart.length > 0 && (
              <div className="absolute -top-1 -right-1 bg-red-500 text-white text-xs w-5 h-5 rounded-full flex items-center justify-center">
                {cart.length}
              </div>
            )}
          </button>
        </div>
      </div>
      
      {/* Sticky Nav */}
      <StickyNavigation />
      
      {/* Main Content */}
      <div className="flex-grow overflow-y-auto">
        {renderScreen()}
      </div>
      
      {/* Search Full Screen View */}
      {searchFocused && renderSearchView()}
      
      {/* Cart Drawer */}
      <CartDrawer />
      
      {/* Modals */}
      <SeatSelectorModal />
      <PremiumInfoModal />
      <TicketSelectorModal />
      <VenueEventsModal />
      <ReviewsModal />
      <AllCategoriesModal />
      <AllVendorsModal />
      <AllItemsModal />
      <AllEventsModal />
      
      {/* Bottom Navigation */}
      <div className="bg-white border-t border-gray-200 flex fixed bottom-0 left-0 right-0 max-w-md mx-auto">
        <button 
          onClick={() => setActiveTab('home')}
          className={`flex flex-col items-center justify-center flex-grow py-2 ${activeTab === 'home' ? 'text-red-600' : 'text-gray-500'}`}
        >
          <Home size={20} />
          <span className="text-xs mt-1">Home</span>
        </button>
        <button 
          onClick={() => setActiveTab('explore')}
          className={`flex flex-col items-center justify-center flex-grow py-2 ${activeTab === 'explore' ? 'text-red-600' : 'text-gray-500'}`}
        >
          <Ticket size={20} />
          <span className="text-xs mt-1">Events</span>
        </button>
        <button 
          onClick={() => setIsCartOpen(true)}
          className={`flex flex-col items-center justify-center flex-grow py-2 ${isCartOpen ? 'text-red-600' : 'text-gray-500'}`}
        >
          <ShoppingCart size={20} />
          <span className="text-xs mt-1">Cart</span>
        </button>
        <button 
          onClick={() => setActiveTab('profile')}
          className={`flex flex-col items-center justify-center flex-grow py-2 ${activeTab === 'profile' ? 'text-red-600' : 'text-gray-500'}`}
        >
          <User size={20} />
          <span className="text-xs mt-1">Profile</span>
        </button>
      </div>
    </div>
  );
};

export default AgapeInteractiveDemo;